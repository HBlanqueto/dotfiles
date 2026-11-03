use crate::detect::{self, Disk};

pub const GIB: u64 = 1024 * 1024 * 1024;
pub const MIN_GIB: u64 = 40;
pub const SHRINK_MARGIN: u64 = 10 * GIB;
pub const EFI_MIN_FREE: u64 = 260 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiskDecision {
    WholeDisk,
    FreeSpace {
        reuse_efi: bool,
    },
    Shrink {
        ntfs: String,
        volume: u64,
        min_free: u64,
        max_free: u64,
        reuse_efi: bool,
    },
    TooSmall {
        missing: u64,
    },
    Blocked(String),
}

impl DiskDecision {
    pub fn clamp_shrink(&self, bytes: u64) -> u64 {
        match self {
            DiskDecision::Shrink {
                min_free, max_free, ..
            } => bytes.clamp(*min_free, *max_free),
            _ => bytes,
        }
    }

    pub fn is_shrink(&self) -> bool {
        matches!(self, DiskDecision::Shrink { .. })
    }
}

pub fn decide(disk: &Disk) -> DiskDecision {
    let need = MIN_GIB * GIB;
    if !disk.available {
        return DiskDecision::Blocked("no disponible".into());
    }
    if disk.size < need {
        return DiskDecision::TooSmall {
            missing: need - disk.size,
        };
    }
    if !disk.windows {
        return DiskDecision::WholeDisk;
    }
    if disk.free >= need {
        let reuse_efi = disk
            .efi()
            .map(|efi| detect::efi_free(efi).is_some_and(|free| free >= EFI_MIN_FREE))
            .unwrap_or(false);
        return DiskDecision::FreeSpace { reuse_efi };
    }
    if disk.test {
        let volume = disk.size;
        let max_free = volume.saturating_sub(need + SHRINK_MARGIN);
        if max_free < need {
            return DiskDecision::TooSmall {
                missing: need - max_free,
            };
        }
        return DiskDecision::Shrink {
            ntfs: format!("{}1", disk.path()),
            volume,
            min_free: need,
            max_free,
            reuse_efi: false,
        };
    }
    let Some(ntfs) = disk.ntfs().into_iter().next().cloned() else {
        return DiskDecision::TooSmall {
            missing: need - disk.free,
        };
    };
    let info = detect::ntfs_info(&ntfs.path());
    if info.bitlocker {
        return DiskDecision::Blocked(
            "Windows usa BitLocker. Desactívalo en Windows o borra el disco.".into(),
        );
    }
    if info.dirty {
        return DiskDecision::Blocked(
            "Windows no se apagó por completo. Desactiva Inicio rápido y apaga Windows (no reinicies)."
                .into(),
        );
    }
    let max_free = ntfs.size.saturating_sub(info.min_bytes + SHRINK_MARGIN);
    if max_free < need {
        return DiskDecision::TooSmall {
            missing: need - max_free,
        };
    }
    let reuse_efi = disk
        .efi()
        .map(|efi| detect::efi_free(efi).is_some_and(|free| free >= EFI_MIN_FREE))
        .unwrap_or(false);
    DiskDecision::Shrink {
        ntfs: ntfs.path(),
        volume: ntfs.size,
        min_free: need,
        max_free,
        reuse_efi,
    }
}

pub fn parse_size(size: &str, total: u64) -> u64 {
    let size = size.trim();
    if let Some(percent) = size.strip_suffix('%') {
        let pct: f64 = percent.parse().unwrap_or(100.0);
        return ((total as f64) * pct / 100.0) as u64;
    }
    let (digits, suffix) = size.split_at(
        size.find(|c: char| !c.is_ascii_digit())
            .unwrap_or(size.len()),
    );
    let value: f64 = digits.parse().unwrap_or(0.0);
    let multiplier = match suffix.trim().to_ascii_uppercase().as_str() {
        "K" | "KB" | "KIB" => 1024_u64,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        _ => 1,
    };
    (value * multiplier as f64) as u64
}

pub fn block_widths(sizes: &[u64], width: usize) -> Vec<usize> {
    let total: u64 = sizes.iter().sum();
    if total == 0 || width == 0 {
        return vec![0; sizes.len()];
    }
    let mut widths: Vec<usize> = sizes
        .iter()
        .map(|&s| ((s as f64 / total as f64) * width as f64).floor() as usize)
        .collect();

    let used: usize = widths.iter().sum();
    let mut leftover = width.saturating_sub(used);
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    order.sort_by(|&a, &b| sizes[b].cmp(&sizes[a]));
    let mut i = 0;
    while leftover > 0 && !order.is_empty() {
        widths[order[i % order.len()]] += 1;
        leftover -= 1;
        i += 1;
    }
    widths
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_strings() {
        let gib = 1024 * 1024 * 1024;
        assert_eq!(parse_size("1G", gib * 10), gib);
        assert_eq!(parse_size("500M", gib * 10), 500 * 1024 * 1024);
        assert_eq!(parse_size("100%", gib * 10), gib * 10);
    }

    #[test]
    fn widths_sum_to_total() {
        let widths = block_widths(&[1024, 3072], 40);
        assert_eq!(widths.iter().sum::<usize>(), 40);
        assert!(widths[1] > widths[0]);
    }
}
