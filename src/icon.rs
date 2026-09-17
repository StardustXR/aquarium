use mime::Mime;
use resvg::{tiny_skia, usvg};
use std::{
	path::{Path, PathBuf},
	sync::LazyLock,
};
use xdg_mime::SharedMimeInfo;

static MIME_DB: LazyLock<SharedMimeInfo> = LazyLock::new(SharedMimeInfo::new);
static ICON_THEME: LazyLock<String> =
	LazyLock::new(|| freedesktop_icons::default_theme_gtk().unwrap_or("hicolor".to_string()));

const ICON_SIZE: u16 = 256;
const ICON_SIZES: [u16; 7] = [256, 128, 96, 64, 48, 32, 24];

pub fn mime_type(path: &Path) -> Mime {
	MIME_DB
		.guess_mime_type()
		.path(path)
		.guess()
		.mime_type()
		.clone()
}

pub fn icon_bitmap(mime: &Mime) -> Option<PathBuf> {
	MIME_DB
		.lookup_icon_names(mime)
		.into_iter()
		.find_map(|name| {
			let icon = lookup_icon(&name)?;
			match icon.extension().is_some_and(|e| e == "svg") {
				true => rasterize(&name, &icon),
				false => Some(icon),
			}
		})
}

fn lookup_icon(name: &str) -> Option<PathBuf> {
	let lookup = |size| {
		freedesktop_icons::lookup(name)
			.with_size(size)
			.with_theme(&ICON_THEME)
			.with_cache()
	};
	ICON_SIZES
		.into_iter()
		.find_map(|size| {
			let path = lookup(size).find()?;
			let png = path.extension().is_some_and(|e| e == "png");
			(png && icon_size(&path).is_none_or(|s| s <= ICON_SIZE)).then_some(path)
		})
		.or_else(|| {
			let path = lookup(ICON_SIZE).force_svg().find()?;
			path.extension().is_some_and(|e| e == "svg").then_some(path)
		})
}

/// themes can hand back an icon bigger than the size asked for, so the size dir is the only way to know what we got
fn icon_size(path: &Path) -> Option<u16> {
	path.iter().rev().filter_map(|c| c.to_str()).find_map(|c| {
		let (w, h) = c.split_once('x')?;
		(w == h).then_some(())?;
		w.parse().ok()
	})
}

fn rasterize(name: &str, svg: &Path) -> Option<PathBuf> {
	let cache = std::env::var_os("XDG_CACHE_HOME")
		.map(PathBuf::from)
		.unwrap_or(std::env::home_dir()?.join(".cache"))
		.join("aquarium/icons");
	let png = cache.join(format!("{}-{name}-{ICON_SIZE}.png", *ICON_THEME));
	if png.exists() {
		return Some(png);
	}

	let tree = usvg::Tree::from_data(&std::fs::read(svg).ok()?, &usvg::Options::default()).ok()?;
	let size = tree.size().to_int_size().scale_to_width(ICON_SIZE as u32)?;
	let mut pixmap = tiny_skia::Pixmap::new(size.width(), size.height())?;
	let scale = size.width() as f32 / tree.size().width();
	resvg::render(
		&tree,
		tiny_skia::Transform::from_scale(scale, scale),
		&mut pixmap.as_mut(),
	);

	std::fs::create_dir_all(&cache).ok()?;
	pixmap.save_png(&png).ok()?;
	Some(png)
}
