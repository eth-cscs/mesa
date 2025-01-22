use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentArray {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Components")]
    pub components: Option<Vec<Component>>,
}

/* impl From<ComponentArray> for ComponentArray {
    fn from(value: ComponentArray) -> Self {
        let component_vec_opt: Option<Vec<Component>> = if let Some(components) = value.components {
            let mut component_vec: Vec<Component> = Vec::with_capacity(components.len());

            components
                .into_iter()
                .for_each(|component: Component| component_vec.push(Component::from(component)));

            Some(component_vec)
        } else {
            None
        };

        ComponentArray {
            components: component_vec_opt,
        }
    }
}

impl Into<ComponentArray> for ComponentArray {
    fn into(self) -> ComponentArray {
        let component_vec_opt: Option<Vec<Component>> = if let Some(components) = self.components {
            let mut component_vec: Vec<Component> = Vec::with_capacity(components.len());

            components
                .into_iter()
                .for_each(|component: Component| component_vec.push(component.into()));

            Some(component_vec)
        } else {
            None
        };

        ComponentArray {
            components: component_vec_opt,
        }
    }
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Component {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ID")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Type")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Flag")]
    pub flag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Enabled")]
    pub enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "SoftwareStatus")]
    pub software_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Role")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "SubRole")]
    pub sub_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "NID")]
    pub nid: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Subtype")]
    pub subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "NetType")]
    pub net_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Arch")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Class")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "ReservationDisabled")]
    pub reservation_disabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "Locked")]
    pub locked: Option<bool>,
}

/* impl From<Component> for Component {
    fn from(value: Component) -> Self {
        Component {
            id: value.id,
            r#type: value.r#type,
            state: value.state,
            flag: value.flag,
            enabled: value.enabled,
            software_status: value.software_status,
            role: value.role,
            sub_role: value.sub_role,
            nid: value.nid,
            subtype: value.subtype,
            net_type: value.net_type,
            arch: value.arch,
            class: value.class,
            reservation_disabled: value.reservation_disabled,
            locked: value.locked,
        }
    }
}

impl Into<Component> for Component {
    fn into(self) -> Component {
        Component {
            id: self.id,
            r#type: self.r#type,
            state: self.state,
            flag: self.flag,
            enabled: self.enabled,
            software_status: self.software_status,
            role: self.role,
            sub_role: self.sub_role,
            nid: self.nid,
            subtype: self.subtype,
            net_type: self.net_type,
            arch: self.arch,
            class: self.class,
            reservation_disabled: self.reservation_disabled,
            locked: self.locked,
        }
    }
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentArrayPostQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "ComponentIDs"))]
    pub component_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "stateonly"))]
    pub state_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "flagonly"))]
    pub falg_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "roleonly"))]
    pub role_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "nidonly"))]
    pub nid_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "softwarestatus"))]
    pub software_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subrole: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nid_start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nid_end: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentArrayPostByNidQuery {
    #[serde(rename(serialize = "NIDRanges"))]
    pub nid_ranges: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "stateonly"))]
    pub state_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "flagonly"))]
    pub falg_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "roleonly"))]
    pub role_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "nidonly"))]
    pub nid_only: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentArrayPostArray {
    #[serde(rename(serialize = "Components"))]
    pub components: Vec<ComponentCreate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Force"))]
    pub force: Option<bool>,
}

/* impl From<ComponentArrayPostArray> for ComponentArrayPostArray {
    fn from(value: ComponentArrayPostArray) -> Self {
        let mut component_vec: Vec<ComponentCreate> = Vec::with_capacity(value.components.len());

        value
            .components
            .into_iter()
            .for_each(|c| component_vec.push(c.into()));

        ComponentArrayPostArray {
            components: component_vec,
            force: value.force,
        }
    }
}

impl Into<ComponentArrayPostArray> for ComponentArrayPostArray {
    fn into(self) -> ComponentArrayPostArray {
        let mut component_vec: Vec<ComponentCreate> = Vec::with_capacity(self.components.len());

        self.components
            .into_iter()
            .for_each(|c| component_vec.push(c.into()));

        ComponentArrayPostArray {
            components: component_vec,
            force: self.force,
        }
    }
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentCreate {
    #[serde(rename(serialize = "ID"))]
    id: String,
    #[serde(rename(serialize = "State"))]
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Flag"))]
    flag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Enabled"))]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "SoftwareStatus"))]
    software_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Role"))]
    role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "SubRole"))]
    sub_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "NID"))]
    nid: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Subtype"))]
    subtype: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "NetType"))]
    net_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Arch"))]
    arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Class"))]
    class: Option<String>,
}

/* impl From<ComponentCreate> for ComponentCreate {
    fn from(value: ComponentCreate) -> Self {
        ComponentCreate {
            id: value.id,
            state: value.state,
            flag: value.flag,
            enabled: value.enabled,
            software_status: value.software_status,
            role: value.role,
            sub_role: value.sub_role,
            nid: value.nid,
            subtype: value.subtype,
            net_type: value.net_type,
            arch: value.arch,
            class: value.class,
        }
    }
}

impl Into<ComponentCreate> for ComponentCreate {
    fn into(self) -> ComponentCreate {
        ComponentCreate {
            id: self.id,
            state: self.state,
            flag: self.flag,
            enabled: self.enabled,
            software_status: self.software_status,
            role: self.role,
            sub_role: self.sub_role,
            nid: self.nid,
            subtype: self.subtype,
            net_type: self.net_type,
            arch: self.arch,
            class: self.class,
        }
    }
} */

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ComponentPut {
    component: ComponentCreate,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "Force"))]
    force: Option<bool>,
}
