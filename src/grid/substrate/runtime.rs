//! 全地圖底板常駐單例：載入 `SWL0`、統計、邊緣線索推導與純 Rust PNG 底圖渲染。

use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock, RwLock};

use flate2::Compression;
use flate2::write::ZlibEncoder;

use super::coords::{DerivedSlope, SensoryEdgeCue, SubstrateRegion};
use super::l0_cell::{L0Cell, SubstrateMaterial, SurfaceWaterKind, landmass};
use super::slots::{U1Plugin, derive_pristine_slots};

const SUBSTRATE_REL_PATH: &str = "data/runtime/landmass_01_substrate.swl0";
/// 推導未開發格槽位用的全域世界種子。
pub const WORLD_SEED: u64 = 0x5357_4C30_0000_0001;

/// 底圖圖層模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtlasLayerMode {
    Terrain,
    Material,
    Water,
    Energy,
    Landmass,
}

impl AtlasLayerMode {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "terrain" => Self::Terrain,
            "material" => Self::Material,
            "water" => Self::Water,
            "energy" => Self::Energy,
            "landmass" => Self::Landmass,
            _ => return None,
        })
    }
}

/// 常駐底板執行期。
pub struct SubstrateRuntime {
    region: SubstrateRegion,
    landmass_01_cells: u64,
    spring_count: u64,
    atlas: RwLock<HashMap<AtlasLayerMode, Arc<Vec<u8>>>>,
}

static GLOBAL: OnceLock<Option<SubstrateRuntime>> = OnceLock::new();

fn locate_file() -> Option<PathBuf> {
    let mut roots: Vec<PathBuf> = vec![PathBuf::from(".")];
    if let Ok(exe) = std::env::current_exe() {
        roots.extend(exe.ancestors().skip(1).take(5).map(PathBuf::from));
    }
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    roots.into_iter().map(|r| r.join(SUBSTRATE_REL_PATH)).find(|p| p.is_file())
}

/// 預熱載入底板（冪等；檔案缺席時記錄為不可用）。
pub fn init_global() {
    let _ = global();
}

/// 取得常駐底板；檔案不可用回傳 `None`。
pub fn global() -> Option<&'static SubstrateRuntime> {
    GLOBAL
        .get_or_init(|| {
            let path = locate_file()?;
            match SubstrateRegion::load_sparse_chunks(&path) {
                Ok(region) => Some(SubstrateRuntime::from_region(region)),
                Err(e) => {
                    tracing::warn!("底板載入失敗 {}: {e}", path.display());
                    None
                }
            }
        })
        .as_ref()
}

/// 供線上九宮格直接呼叫的邊緣微感官線索。
pub fn derive_edge_cue(gx: i32, gy: i32) -> Option<String> {
    global()?.edge_cue(gx, gy).map(|c| c.compact_badge())
}

impl SubstrateRuntime {
    pub fn from_region(region: SubstrateRegion) -> Self {
        let mut landmass_01_cells = 0;
        let mut spring_count = 0;
        for c in region.cells() {
            if c.landmass_id == landmass::DAJIA_WU_TABLELAND {
                landmass_01_cells += 1;
                if c.surface_water() == SurfaceWaterKind::FaultSpring {
                    spring_count += 1;
                }
            }
        }
        Self { region, landmass_01_cells, spring_count, atlas: RwLock::new(HashMap::new()) }
    }

    pub fn region(&self) -> &SubstrateRegion {
        &self.region
    }
    pub fn landmass_01_cells(&self) -> u64 {
        self.landmass_01_cells
    }
    pub fn spring_count(&self) -> u64 {
        self.spring_count
    }
    pub fn l0(&self, gx: i32, gy: i32) -> Option<L0Cell> {
        self.region.get(gx, gy)
    }
    pub fn slope(&self, gx: i32, gy: i32) -> Option<DerivedSlope> {
        self.region.derive_slope(gx, gy)
    }
    pub fn edge_cue(&self, gx: i32, gy: i32) -> Option<SensoryEdgeCue> {
        self.region.derive_sensory_edge_cue(gx, gy)
    }

    /// 取得（或首次渲染並快取）指定圖層之 PNG 位元組。
    pub fn basemap_png(&self, mode: AtlasLayerMode) -> Arc<Vec<u8>> {
        if let Some(hit) = self.atlas.read().ok().and_then(|m| m.get(&mode).cloned()) {
            return hit;
        }
        let png = Arc::new(self.render_png(mode));
        if let Ok(mut m) = self.atlas.write() {
            m.entry(mode).or_insert_with(|| Arc::clone(&png));
        }
        png
    }

    fn render_png(&self, mode: AtlasLayerMode) -> Vec<u8> {
        let (w, h) = (self.region.width, self.region.height);
        let max_gy = self.region.max_gy();
        let mut rgb = Vec::with_capacity(w * h * 3);
        for py in 0..h {
            let gy = max_gy - py as i32;
            for px in 0..w {
                let gx = self.region.min_gx + px as i32;
                rgb.extend_from_slice(&self.pixel(gx, gy, mode));
            }
        }
        encode_png_rgb(w as u32, h as u32, &rgb)
    }

    fn pixel(&self, gx: i32, gy: i32, mode: AtlasLayerMode) -> [u8; 3] {
        let Some(c) = self.region.get(gx, gy) else { return SEA_DEEP };
        let base = match mode {
            AtlasLayerMode::Terrain => self.terrain_px(gx, gy, c),
            AtlasLayerMode::Material => material_px(c),
            AtlasLayerMode::Water => water_px(c),
            AtlasLayerMode::Energy => energy_px(c),
            AtlasLayerMode::Landmass => landmass_px(c),
        };
        if mode != AtlasLayerMode::Landmass && is_closed_neighbor(c) {
            blend(base, FOG, 0.55)
        } else {
            base
        }
    }

    fn terrain_px(&self, gx: i32, gy: i32, c: L0Cell) -> [u8; 3] {
        let sw = c.surface_water();
        if sw == SurfaceWaterKind::OpenSea || c.landmass_id == landmass::SEA {
            return SEA_DEEP;
        }
        if sw == SurfaceWaterKind::FaultSpring {
            return [0, 255, 230];
        }
        let elev = c.elevation_m().unwrap_or(0.0);
        if let Some(rgb) = water_surface_px(sw, c.groundwater_bucket()) {
            return rgb;
        }
        let mut col = match c.material() {
            SubstrateMaterial::IntertidalMudflat => [128, 118, 96],
            SubstrateMaterial::CoastalWindSand => [226, 210, 160],
            _ => elevation_ramp(elev),
        };
        if c.landmass_id == landmass::DAJIA_WU_TABLELAND {
            let slots = derive_pristine_slots(gx, gy, c, WORLD_SEED);
            col = match (slots.u1, slots.v1) {
                (U1Plugin::ThornyBambooRhizome, _) => blend(col, [58, 128, 62], 0.5),
                (U1Plugin::AncientCamphorRoot, _) => blend(col, [28, 92, 52], 0.55),
                (U1Plugin::MiscanthusRoot, _) => blend(col, [176, 178, 110], 0.4),
                _ => col,
            };
        }
        let shade = self.hillshade(gx, gy);
        let mut out = scale(col, shade);
        let band = |g: i32, d: i32| self.region.get(g, d).and_then(|n| n.elevation_m());
        let here = (elev / 25.0).floor();
        let east = band(gx + 1, gy).map(|e| (e / 25.0).floor());
        let south = band(gx, gy - 1).map(|e| (e / 25.0).floor());
        if east.is_some_and(|e| e != here) || south.is_some_and(|e| e != here) {
            out = scale(out, 0.88);
        }
        out
    }

    /// 西北向光源之坡度陰影係數。
    fn hillshade(&self, gx: i32, gy: i32) -> f32 {
        match self.region.derive_slope(gx, gy) {
            Some(s) => 1.0 + (1.2 * 0.707 * (s.dz_dx - s.dz_dy)).clamp(-0.35, 0.35),
            None => 1.0,
        }
    }
}

const SEA_DEEP: [u8; 3] = [22, 44, 98];
const FOG: [u8; 3] = [200, 208, 200];

fn is_closed_neighbor(c: L0Cell) -> bool {
    matches!(
        c.landmass_id,
        landmass::TAICHUNG_BASIN | landmass::HOULI_DAJIA_FAN | landmass::BAGUA_CHANGHUA_PLAIN
    )
}

fn blend(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (f32::from(x) * (1.0 - t) + f32::from(y) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

fn scale(a: [u8; 3], k: f32) -> [u8; 3] {
    let m = |x: u8| (f32::from(x) * k).clamp(0.0, 255.0) as u8;
    [m(a[0]), m(a[1]), m(a[2])]
}

fn elevation_ramp(e: f32) -> [u8; 3] {
    const STOPS: [(f32, [u8; 3]); 6] = [
        (0.0, [150, 190, 120]),
        (60.0, [112, 168, 92]),
        (150.0, [196, 190, 110]),
        (350.0, [176, 138, 92]),
        (800.0, [140, 120, 104]),
        (1500.0, [240, 238, 232]),
    ];
    if e <= STOPS[0].0 {
        return STOPS[0].1;
    }
    for w in STOPS.windows(2) {
        if e <= w[1].0 {
            return blend(w[0].1, w[1].1, (e - w[0].0) / (w[1].0 - w[0].0));
        }
    }
    STOPS[5].1
}

fn water_surface_px(sw: SurfaceWaterKind, bucket: u8) -> Option<[u8; 3]> {
    let depth = f32::from(bucket.min(15)) / 15.0;
    Some(match sw {
        SurfaceWaterKind::DeepRapidRiver => [28, 84, 178],
        SurfaceWaterKind::PerennialStream => blend([84, 150, 224], [40, 110, 200], depth),
        SurfaceWaterKind::IntertidalShallow => [112, 178, 204],
        _ => return None,
    })
}

fn material_px(c: L0Cell) -> [u8; 3] {
    match c.material() {
        SubstrateMaterial::IntertidalMudflat => [110, 98, 80],
        SubstrateMaterial::CoastalWindSand => [236, 216, 140],
        SubstrateMaterial::AlluvialSandyLoam => [120, 190, 96],
        SubstrateMaterial::LateriteGravel => [196, 92, 60],
        SubstrateMaterial::RiverbedQuartzCobble => [210, 220, 236],
        SubstrateMaterial::FaultBareRock => [90, 86, 98],
        SubstrateMaterial::Unknown => SEA_DEEP,
    }
}

fn water_px(c: L0Cell) -> [u8; 3] {
    match c.surface_water() {
        SurfaceWaterKind::FaultSpring => [0, 255, 230],
        SurfaceWaterKind::DeepRapidRiver => [20, 70, 190],
        SurfaceWaterKind::PerennialStream => [70, 150, 235],
        SurfaceWaterKind::SeasonalGully => [190, 170, 120],
        SurfaceWaterKind::OpenSea | SurfaceWaterKind::NoData => SEA_DEEP,
        SurfaceWaterKind::IntertidalShallow => [112, 178, 204],
        SurfaceWaterKind::None => {
            let t = f32::from(c.groundwater_bucket().min(15)) / 15.0;
            blend([40, 120, 200], [236, 226, 196], t)
        }
    }
}

fn energy_px(c: L0Cell) -> [u8; 3] {
    if c.landmass_id == landmass::SEA {
        return [14, 18, 40];
    }
    let t = (f32::from(c.base_energy.clamp(80, 100)) - 80.0) / 20.0;
    if t < 0.5 {
        blend([40, 60, 150], [250, 220, 70], t * 2.0)
    } else {
        blend([250, 220, 70], [230, 50, 40], (t - 0.5) * 2.0)
    }
}

fn landmass_px(c: L0Cell) -> [u8; 3] {
    if matches!(c.surface_water(), SurfaceWaterKind::DeepRapidRiver) {
        return [248, 250, 255];
    }
    match c.landmass_id {
        landmass::DAJIA_WU_TABLELAND => [70, 180, 90],
        landmass::HOULI_DAJIA_FAN => [166, 144, 196],
        landmass::TAICHUNG_BASIN => [220, 170, 90],
        landmass::BAGUA_CHANGHUA_PLAIN => [96, 170, 190],
        landmass::SEA => SEA_DEEP,
        _ => [140, 140, 140],
    }
}

fn png_chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut h = crc32fast::Hasher::new();
    h.update(kind);
    h.update(data);
    out.extend_from_slice(&h.finalize().to_be_bytes());
}

/// 標準 8-bit RGB PNG 編碼（`IHDR + IDAT + IEND`，每列濾波器為 0）。
pub fn encode_png_rgb(width: u32, height: u32, rgb: &[u8]) -> Vec<u8> {
    let row = width as usize * 3;
    let mut raw = Vec::with_capacity((row + 1) * height as usize);
    for y in 0..height as usize {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * row..(y + 1) * row]);
    }
    let mut z = ZlibEncoder::new(Vec::new(), Compression::fast());
    let _ = z.write_all(&raw);
    let idat = z.finish().unwrap_or_default();

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    png_chunk(&mut out, b"IHDR", &ihdr);
    png_chunk(&mut out, b"IDAT", &idat);
    png_chunk(&mut out, b"IEND", &[]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> SubstrateRuntime {
        let mut r = SubstrateRegion::new_filled(0, 0, 4, 3, L0Cell::sea_nodata());
        let land = L0Cell::new(
            Some(80.0),
            1,
            SubstrateMaterial::LateriteGravel,
            SurfaceWaterKind::None,
            20.0,
            90,
        );
        let spring = L0Cell::new(Some(40.0), 1, SubstrateMaterial::RiverbedQuartzCobble, SurfaceWaterKind::FaultSpring, 0.5, 95);
        r.set(1, 1, land);
        r.set(2, 1, spring);
        SubstrateRuntime::from_region(r)
    }

    #[test]
    fn stats_count_landmass_and_springs() {
        let rt = tiny();
        assert_eq!(rt.landmass_01_cells(), 2);
        assert_eq!(rt.spring_count(), 1);
    }

    #[test]
    fn png_is_well_formed_for_every_mode() {
        let rt = tiny();
        for m in ["terrain", "material", "water", "energy", "landmass"] {
            let mode = AtlasLayerMode::parse(m).expect("mode");
            let png = rt.basemap_png(mode);
            assert_eq!(&png[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
            assert_eq!(&png[12..16], b"IHDR");
            assert_eq!(u32::from_be_bytes(png[16..20].try_into().unwrap()), 4);
            assert_eq!(u32::from_be_bytes(png[20..24].try_into().unwrap()), 3);
            assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
        }
        assert!(AtlasLayerMode::parse("x").is_none());
        assert!(Arc::ptr_eq(&rt.basemap_png(AtlasLayerMode::Terrain), &rt.basemap_png(AtlasLayerMode::Terrain)));
    }

    #[test]
    fn closed_neighbors_are_fogged_in_terrain() {
        let mut r = SubstrateRegion::new_filled(0, 0, 2, 1, L0Cell::sea_nodata());
        let mk = |id| L0Cell::new(Some(80.0), id, SubstrateMaterial::LateriteGravel, SurfaceWaterKind::None, 20.0, 90);
        r.set(0, 0, mk(1));
        r.set(1, 0, mk(3));
        let rt = SubstrateRuntime::from_region(r);
        let open = rt.pixel(0, 0, AtlasLayerMode::Material);
        let closed = rt.pixel(1, 0, AtlasLayerMode::Material);
        assert_ne!(open, closed);
    }
}
