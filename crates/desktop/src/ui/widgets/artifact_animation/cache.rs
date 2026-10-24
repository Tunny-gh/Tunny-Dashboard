//! Animation-owned, context-wide image resources. Never uses Gallery URI loaders.
use super::{data, Job};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

const BUDGET: usize = 256 * 1024 * 1024;
const WORKERS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Key {
    scope: (usize, u64, u64),
    path: std::path::PathBuf,
    size: [u32; 2],
}
struct Entry {
    texture: Arc<egui::TextureHandle>,
    bytes: usize,
    used: u64,
    // Keep the Study allocation alive: its address is part of the scope identity.
    _data: Arc<data::Data>,
}
struct Pending {
    key: Key,
    data: Arc<data::Data>,
    bytes: usize,
    job: Job<egui::ColorImage>,
}
#[derive(Default, Clone, Copy, Debug)]
pub(super) struct Stats {
    pub hits: usize,
    pub decodes: usize,
    pub evictions: usize,
    pub obsolete: usize,
    pub retained: usize,
    pub peak: usize,
    pub management_ns: u128,
}
pub(super) struct Cache {
    entries: HashMap<Key, Entry>,
    pending: Vec<Pending>,
    interests: HashMap<u64, Vec<(Key, Arc<data::Data>, usize)>>,
    errors: HashMap<Key, String>,
    clock: u64,
    budget: usize,
    invalidated_scope: Option<(usize, u64, u64)>,
    pub stats: Stats,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            pending: Vec::new(),
            interests: HashMap::new(),
            errors: HashMap::new(),
            clock: 0,
            budget: BUDGET,
            invalidated_scope: None,
            stats: Stats::default(),
        }
    }
}
pub(super) fn shared(ctx: &egui::Context) -> Arc<Mutex<Cache>> {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Arc<Mutex<Cache>>>(egui::Id::new("animation-images"))
            .clone()
    })
}
pub(super) struct Client {
    pub owner: u64,
    pub cache: Arc<Mutex<Cache>>,
    signature: Option<(usize, bool, bool, [u32; 2], usize)>,
}
impl Client {
    pub fn new(ctx: &egui::Context) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self {
            owner: NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            cache: shared(ctx),
            signature: None,
        }
    }
    pub fn suspend(&mut self) {
        if self.signature.take().is_some() {
            self.cache.lock().unwrap().release(self.owner);
        }
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "One bounded image-window request, shared by live and protected export"
    )]
    pub fn update(
        &mut self,
        ctx: &egui::Context,
        data: Arc<data::Data>,
        scope: (usize, u64, u64),
        sequence: &[usize],
        position: usize,
        playing: bool,
        looping: bool,
        size: [u32; 2],
    ) {
        let started = std::time::Instant::now();
        let mut cache = self.cache.lock().unwrap();
        let clients =
            cache.interests.len() + usize::from(!cache.interests.contains_key(&self.owner));
        let bytes = size[0] as usize * size[1] as usize * 8;
        let forward = (cache.budget / bytes.max(1) / 4).clamp(2, 32) + 3;
        let capacity = forward
            .min(cache.budget / bytes.max(1) / (clients + WORKERS))
            .max(1);
        let signature = (position, playing, looping, size, capacity);
        if self.signature != Some(signature) {
            cache.interest(
                self.owner,
                data,
                scope,
                &window(sequence, position, playing, looping, capacity),
                size,
            );
            self.signature = Some(signature);
        }
        cache.drive(ctx);
        cache.stats.management_ns += started.elapsed().as_nanos();
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.cache.lock().unwrap().release(self.owner);
    }
}

/// Quantize upward so small layout jitter does not recreate textures. Never upscale.
pub(super) fn display_size(size: egui::Vec2, scale: f32, side: usize) -> [u32; 2] {
    [size.x, size.y]
        .map(|v| (((v * scale).max(1.0) / 64.0).ceil() as u32 * 64).min(side.clamp(1, 2048) as u32))
}

/// The first item is always demand; remaining items are chronological prefetch.
pub(super) fn window(
    sequence: &[usize],
    position: usize,
    playing: bool,
    looping: bool,
    capacity: usize,
) -> Vec<usize> {
    if sequence.is_empty() {
        return vec![];
    }
    let behind = if capacity >= 4 { 2 } else { 0 };
    let ahead = capacity
        .saturating_sub(1 + behind)
        .min(32)
        .min(sequence.len().saturating_sub(1));
    let offsets: Vec<isize> = if playing {
        (0..=ahead).map(|i| i as isize).chain([-1, -2]).collect()
    } else {
        vec![0, 1, -1, 2, -2, 3, -3, 4, -4]
    };
    let mut result = Vec::new();
    for offset in offsets {
        let p = position as isize + offset;
        let p = if looping {
            p.rem_euclid(sequence.len() as isize)
        } else {
            p
        };
        if p >= 0 && (p as usize) < sequence.len() {
            let index = sequence[p as usize];
            if !result.contains(&index) {
                result.push(index);
            }
        }
    }
    result.truncate(capacity.max(1));
    result
}
impl Cache {
    pub fn maximum_side(&self) -> usize {
        // Leave capacity for the requesting client and both decode slots, even
        // when many large widgets pin different frames simultaneously.
        ((self.budget / 8 / (self.interests.len() + WORKERS + 1)) as f64).sqrt() as usize
    }
    fn key(data: &data::Data, scope: (usize, u64, u64), index: usize, size: [u32; 2]) -> Key {
        Key {
            scope,
            path: data.frames[index].entry.path.clone(),
            size,
        }
    }
    pub fn release(&mut self, owner: u64) {
        self.interests.remove(&owner);
    }
    pub fn invalidate(&mut self, scope: (usize, u64, u64)) {
        self.invalidated_scope = Some(scope);
        let wanted: HashSet<_> = self
            .interests
            .values()
            .flat_map(|v| v.iter().map(|v| v.0.clone()))
            .collect();
        self.entries.retain(|key, e| {
            key.scope == scope || wanted.contains(key) || Arc::strong_count(&e.texture) > 1
        });
        for pending in &self.pending {
            if pending.key.scope != scope && !wanted.contains(&pending.key) {
                pending
                    .job
                    .cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
        }
        self.errors.retain(|key, _| key.scope == scope);
        // Clients release live interests when their Study changes. A protected
        // export retains its old immutable snapshot until it finishes.
        self.update_bytes();
    }
    pub fn interest(
        &mut self,
        owner: u64,
        data: Arc<data::Data>,
        scope: (usize, u64, u64),
        indices: &[usize],
        size: [u32; 2],
    ) {
        let items = indices
            .iter()
            .map(|&i| (Self::key(&data, scope, i, size), data.clone(), i))
            .collect();
        self.interests.insert(owner, items);
        // Reuse overlapping windows. Only actual budget pressure evicts images.
        self.clock += 1;
        for (key, _, _) in &self.interests[&owner] {
            if let Some(entry) = self.entries.get_mut(key) {
                entry.used = self.clock;
            }
        }
    }
    fn update_bytes(&mut self) {
        self.stats.retained = self.entries.values().map(|e| e.bytes).sum::<usize>()
            + self.pending.iter().map(|p| p.bytes).sum::<usize>();
        self.stats.peak = self.stats.peak.max(self.stats.retained);
    }
    fn room(&mut self, bytes: usize) -> bool {
        self.update_bytes();
        if self.stats.retained + bytes <= self.budget {
            return true;
        }
        // Batch eviction with headroom; pinned live/export textures cannot be evicted.
        let target = (self.budget * 4 / 5).min(self.budget.saturating_sub(bytes));
        let mut candidates: Vec<_> = self
            .entries
            .iter()
            .filter(|(_, e)| Arc::strong_count(&e.texture) == 1)
            .map(|(k, e)| (e.used, k.clone()))
            .collect();
        candidates.sort_by_key(|v| v.0);
        for (_, key) in candidates {
            if self.stats.retained <= target {
                break;
            }
            if let Some(entry) = self.entries.remove(&key) {
                self.stats.retained -= entry.bytes;
                self.stats.evictions += 1;
            }
        }
        self.stats.retained + bytes <= self.budget
    }
    pub fn drive(&mut self, ctx: &egui::Context) {
        let wanted: HashSet<_> = self
            .interests
            .values()
            .flat_map(|v| v.iter().map(|v| v.0.clone()))
            .collect();
        if let Some(scope) = self.invalidated_scope {
            self.entries.retain(|key, e| {
                key.scope == scope || wanted.contains(key) || Arc::strong_count(&e.texture) > 1
            });
            if self.entries.keys().all(|k| k.scope == scope) {
                self.invalidated_scope = None;
            }
        }
        let mut i = 0;
        while i < self.pending.len() {
            if !wanted.contains(&self.pending[i].key) {
                self.pending[i]
                    .job
                    .cancel
                    .store(true, std::sync::atomic::Ordering::Relaxed);
            }
            if let Some(result) = self.pending[i].job.poll() {
                let p = self.pending.swap_remove(i);
                if !wanted.contains(&p.key)
                    || p.job.cancel.load(std::sync::atomic::Ordering::Relaxed)
                {
                    self.stats.obsolete += 1;
                } else {
                    match result {
                        Ok(image) => {
                            let bytes = image.size[0] * image.size[1] * 8;
                            let texture = Arc::new(ctx.load_texture(
                                "animation-cached",
                                image,
                                egui::TextureOptions::LINEAR,
                            ));
                            self.entries.insert(
                                p.key,
                                Entry {
                                    texture,
                                    bytes,
                                    used: self.clock,
                                    _data: p.data,
                                },
                            );
                        }
                        Err(error) => {
                            self.errors.insert(p.key, error);
                        }
                    }
                }
            } else {
                i += 1;
            }
        }
        // Demands from every instance precede prefetch, with at most two decoders.
        let mut requests: Vec<_> = self
            .interests
            .values()
            .filter_map(|v| v.first().cloned())
            .collect();
        requests.extend(
            self.interests
                .values()
                .flat_map(|v| v.iter().skip(1).cloned()),
        );
        for (key, data, index) in requests {
            if self.pending.len() == WORKERS {
                break;
            }
            if self.entries.contains_key(&key)
                || self.errors.contains_key(&key)
                || self.pending.iter().any(|p| p.key == key)
            {
                continue;
            }
            let bytes = key.size[0] as usize * key.size[1] as usize * 8;
            if !self.room(bytes) {
                continue;
            }
            let entry = data.frames[index].entry.clone();
            let size = key.size;
            let job = Job::spawn(ctx, move |cancel| {
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("Cancelled".into());
                }
                let image = data::read_image(&entry)?;
                if cancel.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err("Cancelled".into());
                }
                let image = if image.width() > size[0] || image.height() > size[1] {
                    image.thumbnail(size[0], size[1])
                } else {
                    image
                };
                let image = image.into_rgba8();
                Ok(egui::ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ))
            });
            self.pending.push(Pending {
                key,
                data,
                bytes,
                job,
            });
            self.stats.decodes += 1;
        }
        self.update_bytes();
    }
    pub fn get(
        &mut self,
        data: &data::Data,
        scope: (usize, u64, u64),
        index: usize,
        size: [u32; 2],
    ) -> Result<Option<Arc<egui::TextureHandle>>, String> {
        let key = Self::key(data, scope, index, size);
        if let Some(error) = self.errors.get(&key) {
            return Err(error.clone());
        }
        self.clock += 1;
        Ok(self.entries.get_mut(&key).map(|e| {
            e.used = self.clock;
            self.stats.hits += 1;
            e.texture.clone()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::types::Direction;
    use std::time::{Duration, Instant};

    #[test]
    fn chronological_windows_pause_seek_loop_and_selected_export_order() {
        let seq: Vec<_> = (0..100).collect();
        let play = window(&seq, 50, true, false, 35);
        assert_eq!(&play[..4], &[50, 51, 52, 53]);
        assert!(play.contains(&49) && play.contains(&48));
        assert_eq!(
            window(&seq, 50, false, false, 35),
            vec![50, 51, 49, 52, 48, 53, 47, 54, 46]
        );
        assert_eq!(&window(&seq, 99, true, true, 35)[..3], &[99, 0, 1]);
        assert_eq!(window(&[0, 4, 8, 9], 1, true, false, 35), vec![4, 8, 9, 0]);
        assert_eq!(window(&seq, 50, true, true, 1), vec![50]);
        assert_eq!(window(&seq, 50, true, true, 2), vec![50, 51]);
        assert_eq!(window(&seq, 50, false, true, 3), vec![50, 51, 49]);
        assert_eq!(
            display_size(egui::vec2(300.0, 210.0), 2.0, 8192),
            [640, 448]
        );
        assert_eq!(
            display_size(egui::Vec2::splat(10000.0), 2.0, 8192),
            [2048, 2048]
        );
    }

    #[test]
    fn shared_budget_reuses_overlap_instances_resize_and_invalidates_obsolete_work() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let specs: Vec<_> = (0..12).map(|i| (i, i, vec![i as f64], vec![])).collect();
        let study = super::super::tests::study(&specs, vec![Direction::Minimize], false);
        let frames = (0..12)
            .map(|row| {
                let path = dir.path().join(format!("{row}.png"));
                image::RgbImage::from_pixel(64, 64, image::Rgb([row as u8, 0, 0]))
                    .save(&path)
                    .unwrap();
                data::Frame {
                    row,
                    number: row as u32,
                    entry: crate::io::artifacts::ArtifactEntry {
                        path,
                        filename: format!("{row}.png"),
                        mimetype: "image/png".into(),
                    },
                }
            })
            .collect();
        let d = Arc::new(data::Data::new(study, frames));
        let scope = (Arc::as_ptr(&d.study.view.df) as usize, 1, 1);
        let mut c = Cache {
            budget: 64 * 64 * 8 * 6,
            ..Default::default()
        };
        fn settle(c: &mut Cache, ctx: &egui::Context) {
            let start = Instant::now();
            loop {
                c.drive(ctx);
                if c.pending.is_empty() {
                    break;
                }
                assert!(start.elapsed() < Duration::from_secs(10));
                std::thread::sleep(Duration::from_millis(1));
            }
        }
        c.interest(1, d.clone(), scope, &[0, 1, 2], [64, 64]);
        settle(&mut c, &ctx);
        let a = c.get(&d, scope, 1, [64, 64]).unwrap().unwrap();
        let decodes = c.stats.decodes;
        c.interest(2, d.clone(), scope, &[1, 2], [64, 64]);
        settle(&mut c, &ctx);
        let b = c.get(&d, scope, 1, [64, 64]).unwrap().unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        assert_eq!(c.stats.decodes, decodes);
        c.interest(1, d.clone(), scope, &[1, 2, 3], [64, 64]);
        settle(&mut c, &ctx);
        assert_eq!(c.stats.decodes, decodes + 1); // Refill, not rebuild.
        drop(b);
        c.release(2);
        for index in 4..12 {
            c.interest(1, d.clone(), scope, &[index], [64, 64]);
            settle(&mut c, &ctx);
            assert!(c.stats.retained <= c.budget);
        }
        assert!(c.stats.evictions > 0);
        assert!(Arc::ptr_eq(
            &a,
            &c.get(&d, scope, 1, [64, 64]).unwrap().unwrap()
        ));
        drop(a);
        c.interest(1, d.clone(), scope, &[0], [32, 32]);
        settle(&mut c, &ctx);
        assert_eq!(
            c.get(&d, scope, 0, [32, 32]).unwrap().unwrap().size(),
            [32, 32]
        );
        c.interest(1, d.clone(), scope, &[1, 2], [64, 64]);
        c.drive(&ctx);
        let new_scope = (scope.0, 2, 2);
        c.invalidate(new_scope);
        c.interest(1, d.clone(), new_scope, &[11], [64, 64]);
        settle(&mut c, &ctx);
        assert!(c.pending.is_empty());
        assert!(c.entries.keys().all(|k| k.scope == new_scope));
        assert!(c.stats.obsolete > 0);
        assert!(c.stats.peak <= c.budget);
        c.release(1);
        assert!(c.interests.is_empty());
        assert!(Arc::ptr_eq(&shared(&ctx), &shared(&ctx)));
        let mut live = Client::new(&ctx);
        let sequence: Vec<_> = (0..12).collect();
        live.update(
            &ctx,
            d.clone(),
            new_scope,
            &sequence,
            0,
            true,
            true,
            [64, 64],
        );
        live.suspend();
        assert!(live.cache.lock().unwrap().interests.is_empty());
        let mut export = Client::new(&ctx);
        export.update(
            &ctx,
            d.clone(),
            new_scope,
            &[0, 5, 11],
            0,
            true,
            false,
            [64, 64],
        );
        {
            let mut shared = export.cache.lock().unwrap();
            assert!(shared
                .interests
                .values()
                .flatten()
                .all(|v| [0, 5, 11].contains(&v.2)));
            settle(&mut shared, &ctx);
            let snapshot = shared.get(&d, new_scope, 5, [64, 64]).unwrap().unwrap();
            shared.invalidate((new_scope.0, 3, 3));
            assert!(Arc::ptr_eq(
                &snapshot,
                &shared.get(&d, new_scope, 5, [64, 64]).unwrap().unwrap()
            ));
        }
        drop(export);
        live.update(
            &ctx,
            d,
            (new_scope.0, 3, 3),
            &sequence,
            4,
            false,
            false,
            [64, 64],
        );
        assert_eq!(live.cache.lock().unwrap().interests[&live.owner][0].2, 4);
    }
}
