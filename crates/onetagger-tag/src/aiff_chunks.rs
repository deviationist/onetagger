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
//! * otherwise the chunk is grown in place and everything after it is shifted
//!   forward, in 1 MiB blocks copied back-to-front so overlapping regions stay
//!   intact. The file keeps its inode, which matters more than it looks: writing
//!   a temporary file and renaming makes a filesystem watcher report the path as
//!   deleted and then re-created, and a pipeline reacting to those events will
//!   try to drop the track from a library and re-import it. `rust-id3` shifts in
//!   place for the same reason.

use anyhow::{anyhow, Error};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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
        grow_name_in_place(path, &name, new)?;
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

/// Grow the `NAME` chunk in place, shifting everything after it forward.
///
/// Deliberately does **not** write a temporary file and rename. A rename gives
/// the file a new inode, and a filesystem watcher sees that as the old path
/// disappearing: in this setup it produced an `unlink` followed by an `add`,
/// which downstream turned into an attempted delete-from-library and then a
/// re-import of what is really the same file. `rust-id3` has the same
/// constraint and solves it the same way -- its `PlainStorage` grows the region
/// and memmoves the following bytes rather than rebuilding the container -- so
/// matching that behaviour keeps tag writes indistinguishable from the ones
/// OneTagger already performs.
///
/// The cost is atomicity: an interrupted write leaves the file inconsistent.
/// That is the same exposure every existing OneTagger tag write already carries,
/// and the audio payload itself is only ever moved, never rewritten in content.
fn grow_name_in_place(path: &Path, name: &ChunkRef, new: &[u8]) -> Result<(), Error> {
    let mut file = OpenOptions::new().read(true).write(true).open(path)?;
    let old_len = file.metadata()?.len();

    let old_payload = name.size as u64 + (name.size as u64 & 1);
    let new_payload = new.len() as u64 + (new.len() as u64 & 1);
    let delta = new_payload - old_payload;

    // Everything after the NAME chunk shifts forward by `delta`.
    let tail_start = name.offset + HEADER_LEN + old_payload;
    let new_len = old_len + delta;
    file.set_len(new_len)?;

    // Copy backwards, from the end towards `tail_start`. Forwards would
    // overwrite bytes that have not been read yet, since source and destination
    // overlap.
    let mut buf = vec![0u8; 1 << 20];
    let mut remaining = old_len - tail_start;
    while remaining > 0 {
        let block = remaining.min(buf.len() as u64);
        let src = tail_start + remaining - block;
        file.seek(SeekFrom::Start(src))?;
        file.read_exact(&mut buf[..block as usize])?;
        file.seek(SeekFrom::Start(src + delta))?;
        file.write_all(&buf[..block as usize])?;
        remaining -= block;
    }

    // Now the gap is free: write the new chunk header and payload.
    file.seek(SeekFrom::Start(name.offset))?;
    file.write_all(NAME)?;
    file.write_all(&(new.len() as u32).to_be_bytes())?;
    file.write_all(new)?;
    if new.len() % 2 == 1 {
        file.write_all(&[0])?;
    }

    // FORM carries the total payload length, so it grows too.
    let form_size = (new_len - HEADER_LEN) as u32;
    file.seek(SeekFrom::Start(4))?;
    file.write_all(&form_size.to_be_bytes())?;

    file.sync_all()?;
    Ok(())
}
