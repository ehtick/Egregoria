use crate::interaction::{Movable, MovedEvent};
use crate::physics::physics_components::{Drag, Kinematics, Transform};
use cgmath::Vector2;
use imgui::im_str;
use imgui::Ui;
use imgui_inspect::{InspectArgsDefault, InspectRenderDefault};

use crate::cars::car_data::CarComponent;
use crate::cars::IntersectionComponent;
use crate::rendering::meshrender_component::MeshRender;
use specs::prelude::*;
use specs::shrev::EventChannel;
use std::marker::PhantomData;

pub struct ImDragf32;
impl InspectRenderDefault<f32> for ImDragf32 {
    fn render(
        data: &[&f32],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        args: &InspectArgsDefault,
    ) {
        if data.len() != 1 {
            unimplemented!();
        }
        let mut cp = *data[0];
        ui.drag_float(&im_str!("{}", label), &mut cp)
            .speed(args.step.unwrap_or(0.1))
            .build();
    }

    fn render_mut(
        data: &mut [&mut f32],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        args: &InspectArgsDefault,
    ) -> bool {
        if data.len() != 1 {
            unimplemented!();
        }
        ui.drag_float(&im_str!("{}", label), data[0])
            .speed(args.step.unwrap_or(0.1))
            .build()
    }
}

pub struct ImCgVec2;
impl InspectRenderDefault<Vector2<f32>> for ImCgVec2 {
    fn render(
        data: &[&Vector2<f32>],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        _: &InspectArgsDefault,
    ) {
        if data.len() != 1 {
            unimplemented!();
        }
        let x = data[0];
        imgui::InputFloat2::new(ui, &im_str!("{}", label), &mut [x.x, x.y])
            .always_insert_mode(false)
            .build();
    }

    fn render_mut(
        data: &mut [&mut Vector2<f32>],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        args: &InspectArgsDefault,
    ) -> bool {
        if data.len() != 1 {
            unimplemented!();
        }
        let x = &mut data[0];
        let mut conv = [x.x, x.y];
        let changed = ui
            .drag_float2(&im_str!("{}", label), &mut conv)
            .speed(args.step.unwrap_or(0.1))
            .build();
        x.x = conv[0];
        x.y = conv[1];
        changed
    }
}

pub struct ImEntity;
impl InspectRenderDefault<Entity> for ImEntity {
    fn render(
        data: &[&Entity],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        _args: &InspectArgsDefault,
    ) {
        if data.len() != 1 {
            unimplemented!();
        }
        ui.text(&im_str!("{:?} {}", *data[0], label));
    }

    fn render_mut(
        data: &mut [&mut Entity],
        label: &'static str,
        _: &mut World,
        ui: &Ui,
        _: &InspectArgsDefault,
    ) -> bool {
        if data.len() != 1 {
            unimplemented!();
        }
        ui.text(&im_str!("{:?} {}", *data[0], label));
        false
    }
}

pub struct ImVec<T> {
    _phantom: PhantomData<T>,
}
impl<T: InspectRenderDefault<T>> InspectRenderDefault<Vec<T>> for ImVec<T> {
    fn render(
        _data: &[&Vec<T>],
        _label: &'static str,
        _: &mut World,
        _ui: &Ui,
        _args: &InspectArgsDefault,
    ) {
        unimplemented!()
    }

    fn render_mut(
        data: &mut [&mut Vec<T>],
        label: &str,
        w: &mut World,
        ui: &Ui,
        args: &InspectArgsDefault,
    ) -> bool {
        if data.len() != 1 {
            unimplemented!();
        }

        let v = &mut data[0];

        if ui.collapsing_header(&im_str!("{}", label)).build() {
            ui.indent();
            for (i, x) in v.iter_mut().enumerate() {
                let id = ui.push_id(i as i32);
                <T as InspectRenderDefault<T>>::render_mut(&mut [x], "", w, ui, args);
                id.pop(ui);
            }
            ui.unindent();
        }

        false
    }
}

pub struct InspectRenderer<'a, 'b> {
    pub world: &'a mut World,
    pub entity: Entity,
    pub ui: &'b Ui<'b>,
}

fn clone_and_modify<T: Component + Clone>(
    world: &mut World,
    entity: Entity,
    f: impl FnOnce(&mut World, T) -> T,
) {
    let c = world.write_component::<T>().get_mut(entity).cloned();

    if let Some(x) = c {
        let m = f(world, x);
        *world.write_component::<T>().get_mut(entity).unwrap() = m;
    }
}

impl<'a, 'b> InspectRenderer<'a, 'b> {
    fn inspect_component<T: Component + Clone + InspectRenderDefault<T>>(&mut self) {
        let ui = self.ui;
        clone_and_modify(self.world, self.entity, |world, mut x| {
            <T as InspectRenderDefault<T>>::render_mut(
                &mut [&mut x],
                std::any::type_name::<T>().split("::").last().unwrap_or(""),
                world,
                ui,
                &InspectArgsDefault::default(),
            );
            x
        });
    }

    pub fn render(mut self) {
        let ui = self.ui;
        let mut event = None;
        clone_and_modify(self.world, self.entity, |world, mut x: Transform| {
            let mut position = x.position();
            if <ImCgVec2 as InspectRenderDefault<Vector2<f32>>>::render_mut(
                &mut [&mut position],
                "Pos",
                world,
                ui,
                &InspectArgsDefault::default(),
            ) {
                event = Some(position);
            }
            x.set_position(position);
            x
        });

        if let Some(new_pos) = event {
            self.world
                .write_resource::<EventChannel<MovedEvent>>()
                .single_write(MovedEvent {
                    entity: self.entity,
                    new_pos,
                });
        }
        self.inspect_component::<CarComponent>();
        self.inspect_component::<MeshRender>();
        self.inspect_component::<Kinematics>();
        self.inspect_component::<Drag>();
        self.inspect_component::<Movable>();
        self.inspect_component::<IntersectionComponent>();
    }
}
