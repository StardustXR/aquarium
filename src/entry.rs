use crate::{
	Aquarium,
	icon::{icon_bitmap, mime_type},
	upright,
};
use glam::{Quat, Vec3};
use mime::Mime;
use serde::{Deserialize, Serialize};
use stardust_xr_asteroids::{
	ClientState, Context, CustomElement, Element, Entity, Reify, Tasker, Transformable,
	components::{Grabbable, PointerMode, Poseable},
	elements::{Lines, Model, ModelPart, Text},
};
use stardust_xr_fusion::{
	drawable::MaterialParameter,
	fields::Shape,
	spatial::{Spatial, SpatialExt, Transform},
	types::{Posef, Resource},
};
use stardust_xr_molecules::lines::{LineExt, line_from_points};
use std::{
	borrow::Cow, env::current_exe, ffi::OsString, path::PathBuf, process::Command, str::FromStr,
	sync::Arc,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
	pub name: String,
	pub pose: Posef,

	#[serde(skip)]
	pub start_pose: Posef,
	#[serde(skip)]
	pub outside_tank: bool,

	#[serde(skip)]
	pub path: PathBuf,
	#[serde(skip)]
	pub mime: Option<Mime>,
	#[serde(skip)]
	pub icon: Option<PathBuf>,
	#[serde(skip)]
	pub is_dir: bool,
}
impl Entry {
	pub fn new(path: PathBuf, name: String) -> Self {
		let mut entry = Self {
			name,
			pose: Posef::default(),
			start_pose: Posef::default(),
			outside_tank: false,

			path: PathBuf::new(),
			mime: None,
			icon: None,
			is_dir: false,
		};
		entry.rescan(path);
		entry
	}
	pub fn rescan(&mut self, path: PathBuf) {
		self.path = path;
		let mime = mime_type(&self.path);
		self.icon = icon_bitmap(&mime);
		self.mime = Some(mime);
		self.is_dir = self.path.is_dir();
	}
	pub fn open(&self, context: &Context, tank_pose: Posef) {
		let client = context.stardust_client.clone();
		let path = self.path.clone();
		let is_dir = self.is_dir;
		let r = Quat::from(tank_pose.orientation);
		let pose = Transform::from_translation_rotation(
			Vec3::from(tank_pose.position) + r * Vec3::from(self.pose.position),
			r * Quat::from(self.pose.orientation),
		);
		tokio::spawn(async move {
			// the server snapshots the spatial's transform into the token, so it can drop right after
			let token = async {
				let (_spatial, spatial_ref) =
					Spatial::new(&client, client.root(), pose).await.ok()?;
				client.generate_startup_token(spatial_ref).await.ok()
			}
			.await;

			let mut cmd = Command::new(if is_dir && let Ok(current_exe) = current_exe() {
				current_exe.into_os_string()
			} else {
				OsString::from_str("xdg-open").unwrap()
			});
			cmd.arg(path);
			if let Some(token) = token {
				cmd.env("STARDUST_STARTUP_TOKEN", token);
			}
			let Ok(mut child) = cmd.spawn() else {
				return;
			};
			// xdg-open exits right away, reap it so it doesn't sit around as a zombie
			std::thread::spawn(move || child.wait());
		});
	}
}
impl Reify<(Posef, &Arc<Shape>)> for Entry {
	fn reify(
		&self,
		context: &Context,
		_tasks: impl Tasker<Self>,
		tank_pose_shape: (Posef, &Arc<Shape>),
	) -> impl Element<Self> {
		let shape = Shape::Box {
			size: [0.05, 0.05, 0.01].into(),
		};
		Entity::new(shape.clone())
			.pose(if self.outside_tank {
				self.start_pose
			} else {
				self.pose
			})
			.component(Poseable::new({
				let tank_shape = tank_pose_shape.1.clone();
				move |state: &mut Self, mut pose| {
					let sample = tank_shape.sample(pose.position);
					if sample.distance > 0.0 {
						pose.position = sample.closest_point;
					}
					state.pose = upright(pose);
				}
			}))
			.component(
				Grabbable::new({
					let tank_shape = tank_pose_shape.1.clone();
					move |state: &mut Self, pose| {
						let sample = tank_shape.sample(pose.position);
						state.outside_tank = sample.distance > 0.0;
						state.pose = upright(pose);
					}
				})
				.grab_start(|state: &mut Self| {
					state.start_pose = state.pose;
				})
				.grab_stop({
					let context = context.clone();
					let tank_pose = tank_pose_shape.0;
					let tank_shape = tank_pose_shape.1.clone();
					move |state: &mut Self| {
						let sample = tank_shape.sample(state.pose.position);
						if sample.distance > 0.0 {
							state.open(&context, tank_pose);
							state.pose = state.start_pose;
						}
						state.outside_tank = false;
					}
				})
				.pointer_mode(PointerMode::Move),
			)
			.build()
			.child(
				Model::namespaced(Aquarium::APP_ID, "file")
					.part({
						let part = ModelPart::new("Icon");
						if let Some(icon) = &self.icon {
							part.mat_param(
								"diffuse",
								MaterialParameter::Texture {
									value: Resource::Direct {
										path: icon.to_str().unwrap().to_string(),
									},
								},
							)
						} else {
							part
						}
					})
					.build(),
			)
			// .child(Lines::new(lines::shape(shape).into_iter().map(|l| l.thickness(0.0025))).build())
			.child(Text::new(&self.name).pos([0.0, -0.03, 0.0]).build())
			.maybe_child(self.outside_tank.then(|| {
				Lines::new([line_from_points(vec![
					[0.0; 3].into(),
					Vec3::from(self.pose.position) - Vec3::from(self.start_pose.position),
				])
				.thickness(0.005)])
				.build()
			}))
	}
}
