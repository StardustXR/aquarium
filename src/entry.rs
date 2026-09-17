use crate::{
	Aquarium,
	icon::{icon_bitmap, mime_type},
};
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
	types::{Posef, Resource},
};
use stardust_xr_molecules::lines::{self, LineExt};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
	pub name: String,
	pub pose: Posef,

	#[serde(skip)]
	pub path: PathBuf,
	#[serde(skip)]
	pub mime: Option<Mime>,
	#[serde(skip)]
	pub icon: Option<PathBuf>,
}
impl Entry {
	pub fn new(path: PathBuf, name: String) -> Self {
		let mut entry = Self {
			name,
			pose: Posef::default(),
			path: PathBuf::new(),
			mime: None,
			icon: None,
		};
		entry.rescan(path);
		entry
	}
	pub fn rescan(&mut self, path: PathBuf) {
		self.path = path;
		let mime = mime_type(&self.path);
		self.icon = icon_bitmap(&mime);
		self.mime = Some(mime);
	}
}
impl Reify for Entry {
	fn reify(&self, _context: &Context, _tasks: impl Tasker<Self>) -> impl Element<Self> {
		let shape = Shape::Box {
			size: [0.05, 0.05, 0.01].into(),
		};
		Entity::new(shape.clone())
			.pose(self.pose)
			.component(Poseable::new(|state: &mut Self, pose| {
				state.pose = pose;
			}))
			.component(
				Grabbable::new(|state: &mut Self, pose| {
					state.pose = pose;
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
										path: icon
											.to_str()
											.unwrap()
											.to_string(),
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
	}
}
