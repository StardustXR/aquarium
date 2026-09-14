use crate::entry::Entry;
use clap::Parser;
use serde::{Deserialize, Serialize};
use stardust_xr_asteroids::{
	ClientState, Context, CustomElement, Entity, Migrate, Reify, Tasker, Transformable,
	components::{Grabbable, PointerMode},
	elements::{FileWatcher, Lines},
};
use stardust_xr_fusion::{
	fields::Shape,
	project_local_resources,
	types::{Posef, rgba_linear},
};
use stardust_xr_molecules::lines::{self, LineExt};
use std::path::PathBuf;

pub mod entry;

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

#[derive(Debug, Serialize, Deserialize)] // Defining variables used in client
pub struct Aquarium {
	#[serde(skip)]
	pose: Posef,
	path: PathBuf,
	shape: Shape,
	files: Vec<Entry>,
}
impl Default for Aquarium {
	fn default() -> Self {
		Self {
			pose: Posef::default(),
			path: std::env::home_dir().unwrap(),
			shape: Shape::Box {
				size: [0.75, 0.30, 0.30].into(),
			},
			files: vec![],
		}
	}
}
impl Aquarium {
	pub fn load_files(&mut self) {
		self.files = std::fs::read_dir(&self.path)
			.into_iter()
			.flatten()
			.flatten()
			.map(|f| Entry {
				path: f.path(),
				pose: Posef::default(),
			})
			.collect();
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
		self.load_files();
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
			.child(
				FileWatcher::new(self.path.clone(), |state: &mut Self| {
					state.load_files();
				})
				.build(),
			)
			.children(self.files.iter().enumerate().map(|(i, f)| {
				f.reify_substate(_context, _tasks.clone(), move |state: &mut Self| {
					state.files.get_mut(i)
				})
			}))
	}
}
