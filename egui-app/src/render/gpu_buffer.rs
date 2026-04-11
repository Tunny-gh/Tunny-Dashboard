/// GPU バッファの容量定数
pub const DEFAULT_SCATTER_CAPACITY: usize = 50_000;

/// GPU バッファのメモリレイアウト情報（GPU 非依存テスト用）
pub struct GpuBufferLayout {
    pub capacity: usize,
    pub positions_size_bytes: usize,
    pub colors_size_bytes: usize,
    pub sizes_size_bytes: usize,
}

impl GpuBufferLayout {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            positions_size_bytes: capacity * std::mem::size_of::<[f32; 4]>(),
            colors_size_bytes: capacity * std::mem::size_of::<[f32; 4]>(),
            sizes_size_bytes: capacity * std::mem::size_of::<f32>(),
        }
    }

    pub fn total_size_bytes(&self) -> usize {
        self.positions_size_bytes + self.colors_size_bytes + self.sizes_size_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_buffer_layout_sizes() {
        let layout = GpuBufferLayout::new(1000);
        assert_eq!(layout.capacity, 1000);
        assert_eq!(layout.positions_size_bytes, 1000 * 16); // [f32;4] = 16 bytes
        assert_eq!(layout.colors_size_bytes, 1000 * 16);
        assert_eq!(layout.sizes_size_bytes, 1000 * 4); // f32 = 4 bytes
    }

    #[test]
    fn gpu_buffer_layout_total_size() {
        let layout = GpuBufferLayout::new(100);
        assert_eq!(layout.total_size_bytes(), 100 * (16 + 16 + 4));
    }

    #[test]
    fn default_scatter_capacity_is_reasonable() {
        assert!(DEFAULT_SCATTER_CAPACITY >= 5_000);
        assert!(DEFAULT_SCATTER_CAPACITY <= 100_000);
    }
}
