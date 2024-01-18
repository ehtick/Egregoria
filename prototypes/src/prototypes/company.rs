use crate::prototypes::PrototypeBase;
use crate::{
    get_with_err, GoodsCompanyID, Money, NoParent, Power, Prototype, Recipe, Size2D, Zone,
};
use egui_inspect::{debug_inspect_impl, Inspect};
use geom::Vec2;
use mlua::{FromLua, Lua, Table, Value};
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum BuildingGen {
    House,
    Farm,
    CenteredDoor {
        vertical_factor: f32, // 1.0 means that the door is at the bottom, just on the street
    },
    NoWalkway {
        door_pos: Vec2, // door_pos is relative to the center of the building
    },
}
debug_inspect_impl!(BuildingGen);

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq, Eq, Inspect)]
pub enum CompanyKind {
    /// Buyers come to get their goods
    Store,
    /// Buyers get their goods delivered to them
    Factory,
}

#[derive(Debug, Clone)]
pub struct GoodsCompanyPrototype {
    pub base: PrototypeBase,
    pub id: GoodsCompanyID,
    pub bgen: BuildingGen,
    pub kind: CompanyKind,
    pub recipe: Option<Recipe>,
    pub n_trucks: u32,
    pub n_workers: u32,
    pub size: Size2D,
    pub asset_location: String,
    pub price: Money,
    pub power_consumption: Power,
    pub power_production: Power,
    pub zone: Option<Zone>,
}

impl Prototype for GoodsCompanyPrototype {
    type Parent = NoParent;
    type ID = GoodsCompanyID;
    const NAME: &'static str = "goods-company";

    fn from_lua(table: &Table) -> mlua::Result<Self> {
        let base = PrototypeBase::from_lua(table)?;
        Ok(Self {
            id: Self::ID::from(&base.name),
            base,
            bgen: get_with_err(table, "bgen")?,
            kind: get_with_err(table, "kind")?,
            recipe: get_with_err(table, "recipe")?,
            n_trucks: get_with_err::<Option<u32>>(table, "n_trucks")?.unwrap_or(0),
            n_workers: get_with_err::<Option<u32>>(table, "n_workers")?.unwrap_or(0),
            size: get_with_err(table, "size")?,
            asset_location: get_with_err(table, "asset_location")?,
            price: get_with_err(table, "price")?,
            power_consumption: get_with_err(table, "power_consumption")?,
            power_production: get_with_err(table, "power_production")?,
            zone: get_with_err(table, "zone").ok(),
        })
    }

    fn id(&self) -> Self::ID {
        self.id
    }
}

impl Deref for GoodsCompanyPrototype {
    type Target = PrototypeBase;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl<'a> FromLua<'a> for CompanyKind {
    fn from_lua(value: Value<'a>, lua: &'a Lua) -> mlua::Result<Self> {
        let s: String = FromLua::from_lua(value, lua)?;
        match &*s {
            "store" => Ok(Self::Store),
            "factory" => Ok(Self::Factory),
            _ => Err(mlua::Error::external(format!(
                "Unknown company kind: {}",
                s
            ))),
        }
    }
}

impl<'a> FromLua<'a> for BuildingGen {
    fn from_lua(value: Value<'a>, _: &'a Lua) -> mlua::Result<Self> {
        let table = match value {
            Value::String(s) => {
                let s = s.to_str()?;
                return match s {
                    "house" => Ok(Self::House),
                    "farm" => Ok(Self::Farm),
                    _ => Err(mlua::Error::external(format!(
                        "Unknown building gen kind: {}",
                        s
                    ))),
                };
            }
            Value::Table(t) => t,
            _ => Err(mlua::Error::FromLuaConversionError {
                from: value.type_name(),
                to: "BuildingGen",
                message: Some("expected string or table".into()),
            })?,
        };
        let kind = get_with_err::<String>(&table, "kind")?;
        match kind.as_str() {
            "house" => Ok(Self::House),
            "farm" => Ok(Self::Farm),
            "centered_door" => Ok(Self::CenteredDoor {
                vertical_factor: get_with_err(&table, "vertical_factor")?,
            }),
            "no_walkway" => Ok(Self::NoWalkway {
                door_pos: get_with_err(&table, "door_pos")?,
            }),
            _ => Err(mlua::Error::external(format!(
                "Unknown building gen kind: {}",
                kind
            ))),
        }
    }
}