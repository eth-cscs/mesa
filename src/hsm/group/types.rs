use backend_dispatcher::types::{Group as FrontEndGroup, Member as FrontEndMember};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Group {
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<Members>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "exclusiveGroup"))]
    pub exclusive_group: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Members {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ids: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Member {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl Group {
    pub fn new(label: &str, member_vec_opt: Option<Vec<&str>>) -> Self {
        let members_opt = if let Some(member_vec) = member_vec_opt {
            Some(Members {
                ids: Some(member_vec.iter().map(|&id| id.to_string()).collect()),
            })
        } else {
            None
        };

        let group = Self {
            label: label.to_string(),
            description: None,
            tags: None,
            members: members_opt,
            exclusive_group: None,
        };

        group
    }

    /// Get HSM group members
    pub fn get_members(&self) -> Vec<String> {
        // FIXME: try to improve this logic by introducing "smart pointers" or "lifetimes"
        self.members
            .as_ref()
            .and_then(|members| members.ids.clone())
            .unwrap_or(Vec::new())
    }

    /// Get HSM group members
    pub fn get_members_opt(&self) -> Option<Vec<String>> {
        // FIXME: try to improve this logic by introducing "smart pointers" or "lifetimes"
        self.members
            .as_ref()
            .and_then(|members| members.ids.clone())
    }

    /// Add list of xnames to HSM group members
    pub fn add_xnames(&mut self, xnames: &[String]) -> Vec<String> {
        self.members.as_mut().and_then(|members| {
            members
                .ids
                .as_mut()
                .and_then(|ids| Some(ids.extend_from_slice(xnames)))
        });

        self.get_members()
    }
}

impl From<FrontEndGroup> for Group {
    fn from(value: FrontEndGroup) -> Self {
        let mut member_vec = Vec::new();
        let member_vec_backend = value.get_members();

        for member in member_vec_backend {
            member_vec.push(member);
        }

        let members = Members {
            ids: Some(member_vec),
        };

        Group {
            label: value.label,
            description: value.description,
            tags: value.tags,
            members: Some(members),
            exclusive_group: value.exclusive_group,
        }
    }
}

impl Into<FrontEndGroup> for Group {
    fn into(self) -> FrontEndGroup {
        let mut member_vec = Vec::new();
        let member_vec_backend = self.get_members();

        for member in member_vec_backend {
            member_vec.push(member);
        }

        let members = FrontEndMember {
            ids: Some(member_vec),
        };

        FrontEndGroup {
            label: self.label,
            description: self.description,
            tags: self.tags,
            members: Some(members),
            exclusive_group: self.exclusive_group,
        }
    }
}
