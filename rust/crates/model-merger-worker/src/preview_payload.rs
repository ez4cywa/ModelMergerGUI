use model_merger_engine::PreviewData;
use std::fmt;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const MAGIC: &[u8; 4] = b"MMPV";
pub const VERSION: u32 = 1;
const CANCELLATION_CHECK_INTERVAL: usize = 4_096;

#[derive(Debug)]
pub enum PayloadError {
    Io(std::io::Error),
    Cancelled,
}

impl fmt::Display for PayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => source.fmt(formatter),
            Self::Cancelled => formatter.write_str("preview was cancelled"),
        }
    }
}

impl std::error::Error for PayloadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Cancelled => None,
        }
    }
}

impl From<std::io::Error> for PayloadError {
    fn from(source: std::io::Error) -> Self {
        Self::Io(source)
    }
}

pub struct PreviewPayload {
    path: PathBuf,
    length: u64,
}

impl PreviewPayload {
    pub fn create(
        preview: &PreviewData,
        is_cancelled: impl Fn() -> bool,
    ) -> Result<Self, PayloadError> {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "model-merger-preview-{}-{:016x}.bin",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let mut payload = Self { path, length: 0 };
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&payload.path)?;
        let mut writer = BufWriter::new(file);
        check_cancelled(&is_cancelled)?;
        writer.write_all(MAGIC)?;
        write_u32(&mut writer, VERSION)?;
        write_u32(&mut writer, checked_u32(preview.meshes.len())?)?;
        for mesh in &preview.meshes {
            check_cancelled(&is_cancelled)?;
            write_u32(&mut writer, checked_u32(mesh.positions.len())?)?;
            write_u32(&mut writer, checked_u32(mesh.triangle_indices.len())?)?;
            for (index, point) in mesh.positions.iter().enumerate() {
                check_cancelled_at_interval(index, &is_cancelled)?;
                write_point(&mut writer, point)?;
            }
            for (index, normal) in mesh.normals.iter().enumerate() {
                check_cancelled_at_interval(index, &is_cancelled)?;
                write_point(&mut writer, normal)?;
            }
            for (index, value) in mesh.triangle_indices.iter().enumerate() {
                check_cancelled_at_interval(index, &is_cancelled)?;
                write_u32(&mut writer, *value)?;
            }
        }
        writer.flush()?;
        payload.length = writer.get_ref().metadata()?.len();
        Ok(payload)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn length(&self) -> u64 {
        self.length
    }
}

impl Drop for PreviewPayload {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn checked_u32(value: usize) -> Result<u32, std::io::Error> {
    u32::try_from(value).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "preview payload exceeds the supported element count",
        )
    })
}

fn write_u32(writer: &mut impl Write, value: u32) -> Result<(), std::io::Error> {
    writer.write_all(&value.to_le_bytes())
}

fn write_point(writer: &mut impl Write, point: &[f32; 3]) -> Result<(), std::io::Error> {
    for value in point {
        writer.write_all(&value.to_le_bytes())?;
    }
    Ok(())
}

fn check_cancelled_at_interval(
    index: usize,
    is_cancelled: &impl Fn() -> bool,
) -> Result<(), PayloadError> {
    if index.is_multiple_of(CANCELLATION_CHECK_INTERVAL) {
        check_cancelled(is_cancelled)?;
    }
    Ok(())
}

fn check_cancelled(is_cancelled: &impl Fn() -> bool) -> Result<(), PayloadError> {
    if is_cancelled() {
        Err(PayloadError::Cancelled)
    } else {
        Ok(())
    }
}
