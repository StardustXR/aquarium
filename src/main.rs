use crate::entry::Entry;
use clap::Parser;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use stardust_xr_asteroids::{
	ClientState, Context, CustomElement, Entity, Migrate, Reify, Tasker, Transformable,
	components::{Grabbable, PointerMode},
	elements::{FileWatcher, Lines, Model},
};
use stardust_xr_fusion::{
	fields::Shape,
	project_local_resources,
	types::{Posef, rgba_linear},
};
use stardust_xr_molecules::lines::{self, LineExt};
use std::{collections::HashMap, fs::File, io::Write, path::PathBuf};

pub mod entry;
pub mod icon;

#[tokio::main(flavor = "current_thread")]
async fn main() {
	stardust_xr_asteroids::client::run::<Aquarium>(&[&project_local_resources!("res")])
		.await
		.unwrap();
}

#[derive(clap::Parser)]
pub struct Args {
	path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Aquarium {
	#[serde(skip)]
	pose: Posef,
	#[serde(skip)]
	path: PathBuf,
	aquascape_path: Option<PathBuf>,
	shape: Shape,
	entries: HashMap<String, Entry>,
}
impl Default for Aquarium {
	fn default() -> Self {
		Self {
			pose: Posef::default(),
			path: std::env::home_dir().unwrap(),
			aquascape_path: None,
			shape: Shape::Box {
				size: [0.75, 0.30, 0.30].into(),
			},
			entries: HashMap::default(),
		}
	}
}
impl Aquarium {
	pub fn ls(&mut self) -> std::io::Result<()> {
		let entries = std::fs::read_dir(&self.path)?;
		for entry in entries {
			let Ok(entry) = entry else {
				continue;
			};
			let file_name = entry.file_name();
			let Some(file_name) = file_name.to_str() else {
				continue;
			};
			if self.entries.contains_key(file_name) {
				continue;
			}
			self.entries.insert(
				file_name.to_string(),
				Entry::new(entry.path(), file_name.to_string()),
			);
		}

		for entry in self.entries.values_mut() {
			entry.rescan(self.path.join(&entry.name));
		}

		Ok(())
	}

	pub fn config_folder_path(&self) -> PathBuf {
		self.path.join(".aquarium")
	}
	pub fn config_file_path(&self) -> PathBuf {
		self.config_folder_path().join("aquarium.ron")
	}
	pub fn load_config(&mut self) {
		let Ok(config_file_contents) = std::fs::read_to_string(self.config_file_path()) else {
			return;
		};
		let Ok(deserialized) = ron::from_str::<Self>(&config_file_contents) else {
			return;
		};
		let old_self = std::mem::replace(self, deserialized);
		self.path = old_self.path;
		self.pose = old_self.pose;
	}
	pub fn save_config(&mut self) -> std::io::Result<()> {
		let folder_path = self.config_folder_path();
		if !folder_path.exists() {
			std::fs::create_dir(folder_path)?;
		}

		let file_path = self.config_file_path();
		let mut file = File::create(file_path)?;
		let serialized = ron::ser::to_string_pretty(&self, PrettyConfig::new()).unwrap();
		file.write_all(serialized.as_bytes())?;
		Ok(())
	}
}
impl Migrate for Aquarium {
	type Old = Self;
}
impl ClientState for Aquarium {
	const APP_ID: &'static str = "org.stardustxr.Aquarium";

	fn initial_state_update(&mut self) {
		let args = Args::parse();
		let Some(path) = args.path else { return };
		let Ok(canonicalized) = path.canonicalize() else {
			return;
		};
		self.path = canonicalized;
	}

	fn on_start(&mut self, _context: &Context, _tasks: impl Tasker<Self>) {
		self.load_config();
		_ = self.ls();
	}
}
impl Reify for Aquarium {
	fn reify(
		&self,
		_context: &Context,
		_tasks: impl Tasker<Self>,
	) -> impl stardust_xr_asteroids::Element<Self> {
		Entity::new(self.shape.clone())
			.pose(self.pose)
			.component(
				Grabbable::new(|state: &mut Self, pose| {
					state.pose = pose;
				})
				.grab_stop(|state: &mut Self| {
					let _ = state.save_config();
				})
				.pointer_mode(PointerMode::Move),
			)
			.build()
			.child(
				Lines::new(
					lines::shape(self.shape.clone())
						.into_iter()
						.map(|l| l.thickness(0.005).color(rgba_linear!(0.0, 0.1, 0.2, 1.0))),
				)
				.build(),
			)
			.maybe_child(
				self.aquascape_path
					.clone()
					.and_then(|p| {
						Model::direct(self.config_folder_path().join(p).canonicalize().ok()?).ok()
					})
					.map(|p| p.build()),
			)
			.child(
				FileWatcher::new(self.path.clone(), |state: &mut Self| {
					_ = state.ls();
				})
				.build(),
			)
			.stable_children(self.entries.iter().map(|(k, v)| {
				(
					k.clone(),
					v.reify_substate(_context, _tasks.clone(), {
						let k = k.clone();
						move |state: &mut Self| state.entries.get_mut(&k)
					}),
				)
			}))
	}
}
