use crate::state::types::ColormapName;

use super::colormap::ColorMap;

pub fn colormap_from_name(name: &ColormapName) -> ColorMap {
    match name {
        ColormapName::Viridis => ColorMap::viridis(),
        ColormapName::Plasma => ColorMap::plasma(),
        ColormapName::Jet => ColorMap::jet(),
        ColormapName::Turbo => ColorMap::turbo(),
        ColormapName::Inferno => ColorMap::inferno(),
        ColormapName::Coolwarm => ColorMap::coolwarm(),
        ColormapName::Spectral => ColorMap::spectral(),
        ColormapName::Cividis => ColorMap::cividis(),
        ColormapName::BlueYellow => ColorMap::blue_yellow(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colormap_name_to_colormap_jet_boundaries() {
        let jet = colormap_from_name(&ColormapName::Jet);
        assert_eq!(jet.interpolate(0.0), egui::Color32::from_rgb(0, 0, 143));
        assert_eq!(jet.interpolate(1.0), egui::Color32::from_rgb(128, 0, 0));
        assert_eq!(jet.interpolate(-0.1), jet.interpolate(0.0));
        assert_eq!(jet.interpolate(1.1), jet.interpolate(1.0));
    }

    #[test]
    fn colormap_name_to_colormap_each_boundary() {
        for name in ColormapName::all() {
            let cmap = colormap_from_name(name);
            let _ = cmap.interpolate(0.0);
            let _ = cmap.interpolate(1.0);
            let _ = cmap.interpolate(-0.5);
            let _ = cmap.interpolate(1.5);
        }
    }
}
