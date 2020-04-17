use crate::engine_interaction::TimeInfo;
use crate::geometry::Vec2;
use crate::map_model::{Map, Traversable, TraverseDirection, TraverseKind};
use crate::pedestrians::PedestrianComponent;
use crate::physics::{CollisionWorld, Kinematics, PhysicsObject, Transform};
use crate::rendering::meshrender_component::MeshRender;
use crate::utils::{Choose, Restrict};
use cgmath::{Angle, Array, InnerSpace, MetricSpace};
use specs::prelude::*;
use specs::shred::PanicHandler;
use std::borrow::Borrow;

#[derive(Default)]
pub struct PedestrianDecision;

#[derive(SystemData)]
pub struct PedestrianDecisionData<'a> {
    cow: Read<'a, CollisionWorld, PanicHandler>,
    map: Read<'a, Map, PanicHandler>,
    time: Read<'a, TimeInfo>,
    transforms: WriteStorage<'a, Transform>,
    kinematics: WriteStorage<'a, Kinematics>,
    pedestrians: WriteStorage<'a, PedestrianComponent>,
    mr: WriteStorage<'a, MeshRender>,
}

impl<'a> System<'a> for PedestrianDecision {
    type SystemData = PedestrianDecisionData<'a>;

    fn run(&mut self, mut data: Self::SystemData) {
        let cow: &CollisionWorld = data.cow.borrow();
        let map: &Map = data.map.borrow();
        let time: &TimeInfo = data.time.borrow();
        (
            &mut data.transforms,
            &mut data.kinematics,
            &mut data.pedestrians,
            &mut data.mr,
        )
            .join()
            .for_each(|(trans, kin, pedestrian, mr)| {
                objective_update(pedestrian, trans, map);

                let neighbors = cow.query_around(trans.position(), 10.0);

                let objs = neighbors.map(|obj| (obj.pos, cow.get_obj(obj.id)));

                let (desired_v, desired_dir) = calc_decision(pedestrian, trans, kin, objs);

                physics(pedestrian, kin, trans, mr, time, desired_v, desired_dir);
            });
    }
}

pub fn physics(
    pedestrian: &mut PedestrianComponent,
    kin: &mut Kinematics,
    trans: &mut Transform,
    mr: &mut MeshRender,
    time: &TimeInfo,
    desired_velocity: Vec2,
    desired_dir: Vec2,
) {
    assert!(kin.velocity.is_finite());
    assert!(desired_velocity.is_finite());
    let diff = desired_velocity - kin.velocity;
    let mag = diff.magnitude().min(time.delta);
    if mag > 0.0 {
        let lol = diff.normalize_to(mag);
        assert!(lol.x.is_finite());
        kin.velocity += lol;
    }

    let speed = kin.velocity.magnitude();
    pedestrian.walk_anim += 7.0 * speed * time.delta / pedestrian.walking_speed;

    let offset = pedestrian.walk_anim.cos()
        * 0.1
        * (speed * 2.0 - pedestrian.walking_speed).restrict(0.0, 1.0);

    if mr.orders[0].as_rect_mut().offset.y.abs() < 0.25 {
        mr.orders[0].as_rect_mut().offset.x = offset;
        mr.orders[1].as_rect_mut().offset.x = -offset;
    }

    let delta_ang = trans.direction().angle(desired_dir);
    let mut ang = vec2!(1.0, 0.0).angle(trans.direction());

    const ANG_VEL: f32 = 1.0;
    ang.0 += delta_ang
        .0
        .restrict(-ANG_VEL * time.delta, ANG_VEL * time.delta);

    trans.set_direction(vec2!(ang.cos(), ang.sin()));
}

pub fn calc_decision<'a>(
    pedestrian: &mut PedestrianComponent,
    trans: &Transform,
    kin: &Kinematics,
    neighs: impl Iterator<Item = (Vec2, &'a PhysicsObject)>,
) -> (Vec2, Vec2) {
    let objective = match pedestrian.itinerary.get_point() {
        Some(x) => x,
        None => return (vec2!(0.0, 0.0), trans.direction()),
    };

    let position = trans.position();
    let direction = trans.direction();

    let delta_pos: Vec2 = objective - position;
    let dist_to_pos = delta_pos.magnitude();
    let dir_to_pos: Vec2 = if dist_to_pos > 0.0 {
        delta_pos / dist_to_pos
    } else {
        panic!("the fuck");
        vec2!(1.0, 0.0)
    };

    let mut desired_v = vec2!(0.0, 0.0);

    let mut estimated_density = 1.0;

    for (his_pos, his_obj) in neighs {
        if his_pos == position {
            continue;
        }

        let towards_vec = his_pos - position;
        let dist = towards_vec.magnitude();
        let towards_dir: Vec2 = towards_vec / dist;

        let forward_boost = 1.0 + direction.dot(towards_dir).abs();

        estimated_density += 1.0 / (1.0 + dist);

        desired_v += -towards_dir * 1.0 * (-(dist - his_obj.radius) * 0.7).exp() * forward_boost;
    }

    desired_v *= 2.0 / estimated_density.restrict(2.0, 4.0);

    //desired_v += 0.1 * vec2!(rand::random::<f32>(), rand::random()) * desired_v.magnitude();
    desired_v += dir_to_pos * pedestrian.walking_speed;

    let s = desired_v
        .magnitude()
        .restrict(0.0, 1.3 * pedestrian.walking_speed);
    if s > 0.0 {
        desired_v.normalize_to(s);
    }

    (desired_v, (dir_to_pos + kin.velocity).normalize())
}

pub fn objective_update(pedestrian: &mut PedestrianComponent, trans: &Transform, map: &Map) {
    pedestrian.itinerary.check_validity(map);

    if let Some(x) = pedestrian.itinerary.get_point() {
        if x.distance(trans.position()) > 3.0 {
            return;
        }
        pedestrian.itinerary.advance(map);
    }

    if pedestrian.itinerary.is_none() {
        if let Some(closest) = map.closest_lane(trans.position()) {
            pedestrian.itinerary.set_simple(
                Traversable::new(TraverseKind::Lane(closest), TraverseDirection::Forward),
                map,
            );
            pedestrian.itinerary.advance(map);
        }
    }

    if pedestrian.itinerary.has_ended() {
        let t = *unwrap_ret!(pedestrian.itinerary.get_travers());

        match t.kind {
            TraverseKind::Lane(l) => {
                let arrived = &map.intersections()[map.lanes()[l].dst];

                let neighs = arrived.turns_adirectional(l);

                let turn = unwrap_ret!(neighs.choose());

                let direction = if turn.id.src == l {
                    TraverseDirection::Forward
                } else {
                    TraverseDirection::Backward
                };

                pedestrian.itinerary.set_simple(
                    Traversable::new(TraverseKind::Turn(turn.id), direction),
                    map,
                );
            }
            TraverseKind::Turn(turn) => {
                let arrived_at = &map.lanes()[match t.dir {
                    TraverseDirection::Forward => turn.dst,
                    TraverseDirection::Backward => turn.src,
                }];

                let dir_if_take_lane = if arrived_at.src == turn.parent {
                    TraverseDirection::Forward
                } else {
                    TraverseDirection::Backward
                };

                let inter = &map.intersections()[turn.parent];

                let mut traversables = vec![Traversable::new(
                    TraverseKind::Lane(arrived_at.id),
                    dir_if_take_lane,
                )];

                for turn_inter in inter.turns_adirectional(arrived_at.id) {
                    if turn_inter.id == turn {
                        continue;
                    }
                    let direction = if turn_inter.id.src == arrived_at.id {
                        TraverseDirection::Forward
                    } else {
                        TraverseDirection::Backward
                    };

                    traversables.push(Traversable::new(
                        TraverseKind::Turn(turn_inter.id),
                        direction,
                    ));
                }

                pedestrian
                    .itinerary
                    .set_simple(*traversables.choose().unwrap(), map);
            }
        }
    }
}