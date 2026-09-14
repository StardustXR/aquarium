use serde::{Deserialize, Serialize};
use stardust_xr_asteroids::{
	Context, CustomElement, Element, Entity, Reify, Tasker, Transformable,
	components::{Grabbable, PointerMode, Poseable},
	elements::{Lines, Text},
};
use stardust_xr_fusion::{fields::Shape, types::Posef};
use stardust_xr_molecules::lines::{self, LineExt};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Entry {
	pub path: PathBuf,
	pub pose: Posef,
}
impl Reify for Entry {
	fn reify(&self, _context: &Context, _tasks: impl Tasker<Self>) -> impl Element<Self> {
		let shape = Shape::Box {
			size: [0.075, 0.025, 0.005].into(),
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
			.child(Lines::new(lines::shape(shape).into_iter().map(|l| l.thickness(0.0025))).build())
			.child(Text::new(self.path.file_name().unwrap().to_string_lossy()).build())
	}
}
