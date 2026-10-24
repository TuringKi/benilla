//! Browser diagnostics for model atlases and the world at the player's position.

use super::{Equipment, SkinComposites, SkinKey, SkinSections};
use crate::net::{NetEntity, ObjectStore, SelfPlayer};
use benilla_assets::materials::WowModelMaterial;
use benilla_assets::WmoModel;
use benilla_world::collision::WorldCollision;
use benilla_world::terrain_stream::WorldLoadProgress;
use benilla_world::view::WorldCamera;
use benilla_world::wmo_portal::WmoPortalInstance;
use bevy::prelude::*;
use wasm_bindgen::JsValue;

fn record<const N: usize>(fields: [(&str, JsValue); N]) -> JsValue {
    let object = js_sys::Object::new();
    for (key, value) in fields {
        let _ = js_sys::Reflect::set(&object, &key.into(), &value);
    }
    object.into()
}

fn coords(pos: Vec3) -> JsValue {
    benilla_assets::coords::bevy_to_wow(pos)
        .into_iter()
        .map(JsValue::from)
        .collect::<js_sys::Array>()
        .into()
}

pub(super) fn publish(
    time: Res<Time>,
    mut next: Local<f32>,
    player: Option<Res<crate::player::Player>>,
    progress: Option<Res<WorldLoadProgress>>,
    me: Query<
        (
            &ObjectStore,
            &NetEntity,
            Option<&Equipment>,
            Option<&Children>,
        ),
        With<SelfPlayer>,
    >,
    cameras: Query<&GlobalTransform, With<WorldCamera>>,
    parts: Query<&MeshMaterial3d<WowModelMaterial>>,
    materials: Res<Assets<WowModelMaterial>>,
    wmos: Query<&WmoPortalInstance>,
    wmo_assets: Res<Assets<WmoModel>>,
    sections: Option<Res<SkinSections>>,
    mut composites: ResMut<SkinComposites>,
    collision: WorldCollision,
) {
    if time.elapsed_secs() < *next {
        return;
    }
    *next = time.elapsed_secs() + 2.0;
    let Some(window) = web_sys::window() else {
        return;
    };
    if !js_sys::Reflect::has(&window, &"__wenilla_ui_diagnostic".into()).unwrap_or(false) {
        return;
    }
    let body = me
        .single()
        .ok()
        .map(|(store, net, equip, children)| {
            let s = &store.0;
            let key = SkinKey {
                race: s.unit_race().unwrap_or(0),
                sex: s.unit_gender().unwrap_or(0),
                skin: s.player_skin().unwrap_or(0),
                face: s.player_face().unwrap_or(0),
                facial_hair: s.player_facial_hair().unwrap_or(0),
                hair_style: s.player_hair_style().unwrap_or(0),
                hair_color: s.player_hair_color().unwrap_or(0),
                equip: equip.map_or([0; 8], |e| e.bodyslots),
                emblem: equip.and_then(|e| e.emblem),
                tabard_preview: equip.is_some_and(|e| e.tabard_preview),
            };
            let atlas = composites.done.fetch(&key);
            let bound: js_sys::Array = children
                .into_iter()
                .flatten()
                .filter_map(|child| {
                    let handle = &parts.get(*child).ok()?.0;
                    let mat = materials.get(handle)?;
                    Some(JsValue::from(format!(
                        "{:?}: {:?}",
                        handle.id(),
                        mat.base.base_color_texture.as_ref().map(|h| h.id())
                    )))
                })
                .collect();
            record([
                (
                    "display",
                    net.display_id.map_or(JsValue::NULL, JsValue::from),
                ),
                ("look", format!("{key:?}").into()),
                ("atlas", format!("{atlas:?}").into()),
                (
                    "skin",
                    sections
                        .as_ref()
                        .and_then(|t| t.tables.skin_texture(key.race, key.sex, key.skin))
                        .map_or(JsValue::NULL, JsValue::from),
                ),
                ("settled", equip.is_some_and(|e| e.settled).into()),
                ("materials", bound.into()),
            ])
        })
        .unwrap_or(JsValue::NULL);
    let p = player.as_deref();
    let floor = p
        .and_then(|p| collision.ray_body(p.pos + Vec3::Y * 3.0, Dir3::NEG_Y, 100.0))
        .map(|h| h.distance);
    let instances: js_sys::Array = wmos
        .iter()
        .take(32)
        .map(|w| {
            record([
                ("loaded", wmo_assets.get(&w.handle).is_some().into()),
                ("groups", (w.visible.len() as u32).into()),
                (
                    "visible",
                    (w.visible.iter().filter(|v| **v).count() as u32).into(),
                ),
            ])
        })
        .collect();
    let data = record([
        ("gpuMesh", true.into()),
        ("pos", p.map_or(JsValue::NULL, |p| coords(p.pos))),
        ("settling", p.is_some_and(|p| p.settling).into()),
        ("worldStale", p.is_some_and(|p| p.world_stale).into()),
        (
            "camera",
            cameras
                .single()
                .ok()
                .map_or(JsValue::NULL, |t| coords(t.translation())),
        ),
        (
            "floorDistanceFrom3AboveFeet",
            floor.map_or(JsValue::NULL, JsValue::from),
        ),
        (
            "progress",
            progress.map_or(JsValue::NULL, |p| {
                record([
                    ("ready", (p.ready as u32).into()),
                    ("total", (p.total as u32).into()),
                    ("presentable", p.presentable().into()),
                    ("collidersPending", (p.colliders_pending as u32).into()),
                ])
            }),
        ),
        ("body", body),
        ("wmos", instances.into()),
    ]);
    let _ = js_sys::Reflect::set(&window, &"__wenilla_render_diagnostic".into(), &data);
}
