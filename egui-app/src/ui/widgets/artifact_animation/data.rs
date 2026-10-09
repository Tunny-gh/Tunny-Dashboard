use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use crate::io::artifacts::ArtifactEntry;
use crate::state::types::{Direction, StudyContext};
use tunny_core::dataframe::FeasibilityState;

#[derive(Clone, Debug)]
pub(super) struct Frame {
    pub row: usize,
    pub number: u32,
    pub entry: ArtifactEntry,
}

/// Metadata recognizes stored images; the decoder verifies the actual file format.
pub(super) fn png_jpeg(entry: &ArtifactEntry) -> bool {
    let mime = entry
        .mimetype
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match mime.as_str() {
        "image/png" | "image/jpeg" | "image/jpg" => true,
        mime if mime.starts_with("image/") => false,
        _ => match std::path::Path::new(&entry.filename)
            .extension()
            .and_then(|e| e.to_str())
        {
            Some(extension) => ["png", "jpg", "jpeg"]
                .iter()
                .any(|v| extension.eq_ignore_ascii_case(v)),
            // An extensionless legacy entry may have no metadata. read_image
            // must still verify a PNG/JPEG header and successful decoding.
            None => true,
        },
    }
}

pub(super) fn read_image(entry: &ArtifactEntry) -> Result<image::DynamicImage, String> {
    let reader = image::ImageReader::open(&entry.path)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| format!("Cannot read {}: {e}", entry.filename))?;
    if !png_jpeg(entry)
        || !matches!(
            reader.format(),
            Some(image::ImageFormat::Png | image::ImageFormat::Jpeg)
        )
    {
        return Err("Artifact is not a PNG/JPEG image".into());
    }
    reader
        .decode()
        .map_err(|e| format!("Cannot read {}: {e}", entry.filename))
}

pub(super) fn frame_list(
    study: &StudyContext,
    artifacts: &HashMap<u32, Vec<ArtifactEntry>>,
    index: usize,
    cancel: &AtomicBool,
) -> Vec<Frame> {
    let mut frames = Vec::new();
    for (row, id) in study.view.trial_ids.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let Some(entry) = artifacts.get(id).and_then(|v| v.get(index)) else {
            continue;
        };
        let Some(number) = study.view.df.get_trial_number(row) else {
            continue;
        };
        if read_image(entry).is_ok() {
            frames.push(Frame {
                row,
                number,
                entry: entry.clone(),
            });
        }
    }
    frames.sort_by_key(|f| (f.number, f.row));
    frames
}

#[derive(Clone)]
pub(super) struct Data {
    pub study: StudyContext,
    pub frames: Vec<Frame>,
    /// Every observed row, including artifactless rows, ordered by Study-local number.
    pub rows: Vec<usize>,
    pub objectives: Vec<Vec<f64>>,
    pub bounds: Vec<(f64, f64)>,
    pub trial_bounds: (f64, f64),
    pub minimize: Vec<bool>,
}

impl Data {
    pub fn new(study: StudyContext, frames: Vec<Frame>) -> Self {
        let names = &study.meta.objective_names;
        let mut rows: Vec<_> = (0..study.view.row_count()).collect();
        rows.sort_by_key(|&r| study.view.df.get_trial_number(r).unwrap_or(u32::MAX));
        let objectives = (0..study.view.row_count())
            .map(|r| {
                names
                    .iter()
                    .map(|name| {
                        study
                            .view
                            .df
                            .objective_column(name)
                            .and_then(|v| v.get(r))
                            .copied()
                            .unwrap_or(f64::NAN)
                    })
                    .collect()
            })
            .collect();
        let bounds = names
            .iter()
            .map(|name| {
                range(
                    study
                        .view
                        .df
                        .objective_column(name)
                        .unwrap_or(&[])
                        .iter()
                        .copied(),
                )
            })
            .collect();
        let minimize = names
            .iter()
            .enumerate()
            .map(|(i, _)| !matches!(study.meta.directions.get(i), Some(Direction::Maximize)))
            .collect();
        let trial_bounds = range(
            rows.iter()
                .filter_map(|&row| study.view.df.get_trial_number(row))
                .map(f64::from),
        );
        Self {
            study,
            frames,
            rows,
            objectives,
            bounds,
            trial_bounds,
            minimize,
        }
    }
    pub fn number(&self, row: usize) -> u32 {
        self.study
            .view
            .df
            .get_trial_number(row)
            .expect("observed trial number")
    }
}

pub(super) fn range(values: impl Iterator<Item = f64>) -> (f64, f64) {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values.filter(|v| v.is_finite()) {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if !lo.is_finite() {
        return (0.0, 1.0);
    }
    let margin = if lo == hi {
        lo.abs().max(1.0) * 0.05
    } else {
        (hi * 0.05 - lo * 0.05).abs()
    };
    ((lo - margin).max(-f64::MAX), (hi + margin).min(f64::MAX))
}

pub(super) struct Prefix {
    pub number: u32,
    pub rows: Vec<usize>,
    pub ranks: HashMap<usize, u32>,
}

impl Prefix {
    pub fn new(data: &Data, number: u32) -> Self {
        let rows: Vec<_> = data
            .rows
            .iter()
            .copied()
            .filter(|&r| data.number(r) <= number)
            .collect();
        // Exclude invalid objectives across *all* axes, not just the projection.
        let mut valid: Vec<_> = rows
            .iter()
            .copied()
            .filter(|&r| {
                !data.objectives[r].is_empty() && data.objectives[r].iter().all(|v| v.is_finite())
            })
            .collect();
        // Match DataFrame input order for constrained-rank tie-breaking;
        // history remains ordered by Study-local trial number.
        valid.sort_unstable();
        let feasibility = data.study.view.feasibility();
        let feasible: Vec<_> = valid
            .iter()
            .copied()
            .filter(|&r| feasibility.state(r) == FeasibilityState::Feasible)
            .collect();
        let objectives: Vec<_> = feasible
            .iter()
            .map(|&r| data.objectives[r].clone())
            .collect();
        let sorted = tunny_core::pareto::nd_sort(&objectives, &data.minimize);
        let mut ranks: HashMap<_, _> = feasible.into_iter().zip(sorted).collect();
        let next = ranks.values().copied().max().unwrap_or(0) + 1;
        let sums = data.study.view.numeric_column("constraint_sum");
        let mut infeasible: Vec<_> = valid
            .iter()
            .copied()
            .filter(|&r| feasibility.state(r) == FeasibilityState::Infeasible)
            .collect();
        infeasible.sort_by(|&a, &b| {
            let sum = |r| sums.and_then(|v| v.get(r)).copied().unwrap_or(0.0);
            sum(a)
                .partial_cmp(&sum(b))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let count = infeasible.len() as u32;
        for (i, r) in infeasible.into_iter().enumerate() {
            ranks.insert(r, next + i as u32);
        }
        for (i, r) in valid
            .into_iter()
            .filter(|&r| feasibility.state(r) == FeasibilityState::Unverified)
            .enumerate()
        {
            ranks.insert(r, next + count + i as u32);
        }
        Self {
            number,
            rows,
            ranks,
        }
    }
}

/// Keep a bounded number of prefixes, not an O(trials²) collection of snapshots.
#[derive(Default)]
pub(super) struct PrefixCache(pub Vec<Arc<Prefix>>);
impl PrefixCache {
    pub fn get(&self, number: u32) -> Option<Arc<Prefix>> {
        self.0.iter().find(|p| p.number == number).cloned()
    }
    pub fn insert(&mut self, prefix: Arc<Prefix>) {
        if self.0.len() == 8 {
            self.0.remove(0);
        }
        self.0.push(prefix);
    }
}
