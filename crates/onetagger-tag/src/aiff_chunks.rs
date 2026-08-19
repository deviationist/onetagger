//! AIFF chunk-level surgery, for keeping the `NAME` chunk in step with ID3 `TIT2`.
//!
//! AIFF can carry a title in two independent places: the IFF `NAME` chunk near the
//! head of the file, and an ID3 `TIT2` frame inside an `ID3 ` chunk (conventionally
//! at the tail). Neither spec says which wins, and consumers disagree — Plex reads
//! `NAME` and ignores `TIT2` when both exist. Since OneTagger writes only ID3, a
//! title edit leaves the file internally contradictory and Plex keeps serving the
//! old title through any number of rescans.
//!
//! This module only ever *updates an existing* `NAME`. It deliberately never
//! creates one: a file without `NAME` is unambiguous today, because every consumer
//! falls back to ID3, and adding a second title field would manufacture exactly the
//! desync this is meant to remove.
//!
//! `NAME` sits before the audio, so growing it shifts the whole `SSND` payload —
//! typically 50-130 MB in a real library. Two paths handle that:
//!
//! * the new title fits the existing chunk, so the payload is patched in place and
//!   space-padded, leaving every byte offset untouched;
//! * otherwise the file is rebuilt into a temporary file and atomically renamed,
//!   streaming payloads rather than buffering them, so a 130 MB track never lands
//!   in memory and a crash mid-write cannot destroy the audio.

use anyhow::{anyhow, Error};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;

const HEADER_LEN: u64 = 8;
const FORM: &[u8; 4] = b"FORM";
const NAME: &[u8; 4] = b"NAME";

/// One top-level chunk: its tag, where its payload starts, and how long it is.
#[derive(Debug, Clone)]
struct ChunkRef {
    tag: [u8; 4],
    /// Offset of the chunk header (the tag), not the payload.
    offset: u64,
    /// Payload length, excluding the pad byte an odd length implies.
    size: u32,
}

impl ChunkRef {
    /// Total bytes the chunk occupies, header and IFF word-alignment pad included.
    fn total_len(&self) -> u64 {
        HEADER_LEN + self.size as u64 + (self.size as u64 & 1)
    }
}

/// Read the top-level chunk table without touching payloads.
///
/// Accepts both `AIFF` and `AIFC`; the form type is not validated beyond the
/// `FORM` container, since this only rewrites chunks it recognises and copies
/// everything else through untouched.
fn read_chunks(file: &mut File) -> Result<Vec<ChunkRef>, Error> {
    let file_len = file.metadata()?.len();
    let mut header = [0u8; 12];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut header)?;
    if &header[0..4] != FORM {
        return Err(anyhow!("not an IFF FORM container"));
    }

    let mut chunks = vec![];
    // 12 = FORM header (8) + form type (4)
    let mut offset: u64 = 12;
    while offset + HEADER_LEN <= file_len {
        let mut buf = [0u8; 8];
        file.seek(SeekFrom::Start(offset))?;
        if file.read_exact(&mut buf).is_err() {
            break;
        }
        let size = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let chunk = ChunkRef {
            tag: [buf[0], buf[1], buf[2], buf[3]],
            offset,
            size,
        };
        // A size that runs past the end means a truncated or lying file. Stop
        // rather than trusting it -- better to leave NAME alone than to rebuild
        // a container from a bad chunk table.
        if offset + chunk.total_len() > file_len {
            break;
        }
        offset += chunk.total_len();
        chunks.push(chunk);
    }
    Ok(chunks)
}

/// Update the `NAME` chunk to `title`, if the file has one and it disagrees.
///
/// Returns `Ok(true)` when the file was modified. Absent `NAME`, or a `NAME` that
/// already matches, is `Ok(false)` — both are the common case and neither is an
/// error.
pub fn sync_name_chunk(path: impl AsRef<Path>, title: &str) -> Result<bool, Error> {
    let path = path.as_ref();
    let mut file = File::open(path)?;
    let chunks = read_chunks(&mut file)?;

    let name = match chunks.iter().find(|c| &c.tag == NAME) {
        Some(c) => c.clone(),
        // No NAME chunk: nothing to keep in sync, and we never create one.
        None => return Ok(false),
    };

    let mut current = vec![0u8; name.size as usize];
    file.seek(SeekFrom::Start(name.offset + HEADER_LEN))?;
    file.read_exact(&mut current)?;

    // Compare trimmed: padded chunks are common in the wild, and consumers trim
    // trailing whitespace, so " Foo " and "Foo" are the same title in practice.
    let current_str = String::from_utf8_lossy(&current);
    if current_str.trim_end() == title {
        return Ok(false);
    }

    let new = title.as_bytes();
    if new.len() as u32 <= name.size {
        patch_in_place(path, &name, new)?;
    } else {
        rewrite_with_name(path, &chunks, &name, new)?;
    }
    Ok(true)
}

/// Overwrite the payload and pad with spaces. No offset changes, so the audio is
/// never touched and the write is a few bytes regardless of file size.
fn patch_in_place(path: &Path, name: &ChunkRef, new: &[u8]) -> Result<(), Error> {
    let mut file = OpenOptions::new().write(true).open(path)?;
    let mut payload = vec![b' '; name.size as usize];
    payload[..new.len()].copy_from_slice(new);
    file.seek(SeekFrom::Start(name.offset + HEADER_LEN))?;
    file.write_all(&payload)?;
    file.sync_all()?;
    Ok(())
}

/// Rebuild the container with a longer `NAME`, streaming every other chunk through
/// byte-for-byte, then atomically rename over the original.
fn rewrite_with_name(
    path: &Path,
    chunks: &[ChunkRef],
    name: &ChunkRef,
    new: &[u8],
) -> Result<(), Error> {
    let src = File::open(path)?;
    let mut reader = BufReader::new(src);

    let tmp_path = path.with_extension(format!(
        "{}.1t-tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("aiff")
    ));
    let tmp = File::create(&tmp_path)?;
    let mut writer = BufWriter::new(tmp);

    // Recompute the FORM payload size: 4 bytes of form type, plus every chunk at
    // its new length.
    let mut form_size: u64 = 4;
    for c in chunks {
        form_size += if c.offset == name.offset {
            HEADER_LEN + new.len() as u64 + (new.len() as u64 & 1)
        } else {
            c.total_len()
        };
    }
    if form_size > u32::MAX as u64 {
        return Err(anyhow!("AIFF would exceed the 4 GiB FORM limit"));
    }

    let mut form_header = [0u8; 12];
    reader.seek(SeekFrom::Start(0))?;
    reader.read_exact(&mut form_header)?;
    form_header[4..8].copy_from_slice(&(form_size as u32).to_be_bytes());
    writer.write_all(&form_header)?;

    for c in chunks {
        if c.offset == name.offset {
            writer.write_all(NAME)?;
            writer.write_all(&(new.len() as u32).to_be_bytes())?;
            writer.write_all(new)?;
            if new.len() % 2 == 1 {
                writer.write_all(&[0])?;
            }
        } else {
            // Stream header and payload; SSND can be hundreds of megabytes and
            // must never be buffered whole.
            reader.seek(SeekFrom::Start(c.offset))?;
            let mut chunk_reader = (&mut reader).take(c.total_len());
            std::io::copy(&mut chunk_reader, &mut writer)?;
        }
    }

    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);

    // Preserve ownership and mode: these files live on a shared NFS dataset where
    // group-writability is what lets other containers touch them.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(path) {
            let _ = std::fs::set_permissions(
                &tmp_path,
                std::fs::Permissions::from_mode(meta.permissions().mode()),
            );
        }
    }

    std::fs::rename(&tmp_path, path)?;
    Ok(())
}
