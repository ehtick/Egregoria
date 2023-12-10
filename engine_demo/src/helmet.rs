use crate::DemoElement;
use engine::meshload::load_mesh;
use engine::{Context, FrameContext, InstancedMesh, InstancedMeshBuilder, MeshInstance};
use geom::{vec3, Camera, InfiniteFrustrum, LinearColor, Vec3};

pub struct Helmet {
    mesh: Option<InstancedMesh>,
}

impl DemoElement for Helmet {
    fn name(&self) -> &'static str {
        "Helmet"
    }

    fn init(ctx: &mut Context) -> Self {
        let gfx = &mut ctx.gfx;

        let Ok(mesh) = load_mesh(gfx, "DamagedHelmet.glb") else {
            return Self { mesh: None };
        };
        let mut i = InstancedMeshBuilder::<true>::new(mesh);
        i.instances.push(MeshInstance {
            pos: vec3(0.0, 10.0, 0.0),
            dir: Vec3::X * 3.0,
            tint: LinearColor::WHITE,
        });
        let mesh = i.build(gfx).unwrap();

        Self { mesh: Some(mesh) }
    }

    fn update(&mut self, _ctx: &mut Context) {}

    fn render(&mut self, fc: &mut FrameContext, _cam: &Camera, _frustrum: &InfiniteFrustrum) {
        fc.draw(self.mesh.clone());
    }
}