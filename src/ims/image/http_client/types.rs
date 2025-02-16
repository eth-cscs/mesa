use backend_dispatcher::types::ims::{
    Image as FrontEndImage, ImsImageRecord2Update as FrontEndImsImageRecord2Update,
    Link as FrontEndLink,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ImsImageRecord2Update {
    pub link: Link,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl From<FrontEndImsImageRecord2Update> for ImsImageRecord2Update {
    fn from(frontend_ims_image_record2_update: FrontEndImsImageRecord2Update) -> Self {
        Self {
            link: frontend_ims_image_record2_update.link.into(),
            arch: frontend_ims_image_record2_update.arch,
        }
    }
}

impl Into<FrontEndImsImageRecord2Update> for ImsImageRecord2Update {
    fn into(self) -> FrontEndImsImageRecord2Update {
        FrontEndImsImageRecord2Update {
            link: self.link.into(),
            arch: self.arch,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Link {
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    pub r#type: String,
}

impl From<FrontEndLink> for Link {
    fn from(frontend_link: FrontEndLink) -> Self {
        Self {
            path: frontend_link.path,
            etag: frontend_link.etag,
            r#type: frontend_link.r#type,
        }
    }
}

impl Into<FrontEndLink> for Link {
    fn into(self) -> FrontEndLink {
        FrontEndLink {
            path: self.path,
            etag: self.etag,
            r#type: self.r#type,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Image {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link: Option<Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
}

impl From<FrontEndImage> for Image {
    fn from(frontend_image: FrontEndImage) -> Self {
        Self {
            id: frontend_image.id,
            created: frontend_image.created,
            name: frontend_image.name,
            link: frontend_image.link.map(|link| link.into()),
            arch: frontend_image.arch,
        }
    }
}

impl Into<FrontEndImage> for Image {
    fn into(self) -> FrontEndImage {
        FrontEndImage {
            id: self.id,
            created: self.created,
            name: self.name,
            link: self.link.map(|link| link.into()),
            arch: self.arch,
        }
    }
}
