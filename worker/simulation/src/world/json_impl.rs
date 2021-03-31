use serde_json::json;

use super::World;
use crate::{
    indices::WorldPosition,
    prelude::{Axial, Hexagon},
};
use std::collections::HashMap;

fn pos_to_string(pos: Axial) -> String {
    format!("{};{}", pos.q, pos.r)
}

pub fn json_serialize_resources(world: &World) -> serde_json::Value {
    let resources = world
        .entities
        .iterby_resource()
        .filter_map(|payload| payload.pos.map(|_| payload))
        .fold(HashMap::new(), |mut map, payload| {
            let room = payload.pos.unwrap().0.room;
            let room = pos_to_string(room);
            map.entry(room).or_insert_with(Vec::new).push(payload);
            map
        });
    serde_json::to_value(&resources).unwrap()
}

pub fn json_serialize_terrain(world: &World) -> serde_json::Value {
    let terrain = &world.positions.point_terrain;
    let bounds = Hexagon::from_radius(world.config.room_properties.unwrap_value().radius as i32);
    let points = bounds.iter_points().collect::<Vec<_>>();
    let terrain = terrain
        .iter_rooms()
        .flat_map(|(room_id, room)| {
            room.iter()
                .map(move |(pos, t)| {
                    (
                        WorldPosition {
                            room: room_id.0,
                            pos,
                        },
                        t,
                    )
                })
                .map(|(pos, terrain)| (pos_to_string(pos.room), terrain))
        })
        .fold(HashMap::new(), |mut map, (room, payload)| {
            map.entry(room).or_insert_with(Vec::new).push(payload);
            map
        });
    serde_json::json!({
        "roomLayout": points,
        "roomTerrain": terrain
    })
}

pub fn json_serialize_structures(world: &World) -> serde_json::Value {
    let structures = world
        .entities
        .iterby_structure()
        .filter_map(|payload| payload.pos.map(|_| payload))
        .fold(HashMap::new(), |mut map, payload| {
            let room = payload.pos.unwrap().0.room;
            let room = pos_to_string(room);
            map.entry(room).or_insert_with(Vec::new).push(payload);
            map
        });
    serde_json::to_value(&structures).unwrap()
}

pub fn json_serialize_bots(world: &World) -> serde_json::Value {
    let bots = world
        .entities
        .iterby_bot()
        .filter_map(|mut payload| {
            payload.pathcache = None;
            payload.pos.map(|_| payload)
        })
        .fold(HashMap::new(), |mut map, payload| {
            let room = payload.pos.unwrap().0.room;
            let room = pos_to_string(room);
            map.entry(room).or_insert_with(Vec::new).push(payload);
            map
        });
    serde_json::to_value(&bots).unwrap()
}

pub fn json_serialize_users(world: &World) -> serde_json::Value {
    let users = world
        .user
        .iterby_user()
        .map(|pl| (pl.__id, pl))
        .collect::<HashMap<_, _>>();

    serde_json::to_value(&users).unwrap()
}

pub fn json_serialize_rooms(world: &World) -> serde_json::Value {
    let rooms = world
        .room
        .iterby_rooms()
        .fold(HashMap::new(), |mut map, payload| {
            map.insert(
                pos_to_string(payload.__id),
                json!({
                    "owner": &payload.owner
                }),
            );
            map
        });

    serde_json::to_value(&rooms).unwrap()
}
