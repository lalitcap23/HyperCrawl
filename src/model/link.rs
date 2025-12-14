use anyhow::{anyhow, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use super::Image;

pub type LinkId = Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Link {
    pub id: LinkId,
    pub url: String,
    #[serde(serialize_with = "serialize_hashset")]
    pub parents: HashSet<LinkId>,
    #[serde(serialize_with = "serialize_hashset")]
    pub children: HashSet<LinkId>,
    pub images: Vec<Image>,
    pub titles: Vec<String>,
}

fn serialize_hashset<S>(set: &HashSet<LinkId>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let vec: Vec<&LinkId> = set.iter().collect();
    vec.serialize(serializer)
}

impl Default for Link {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            url: String::new(),
            parents: HashSet::new(),
            children: HashSet::new(),
            images: Vec::new(),
            titles: Vec::new(),
        }
    }
}

impl Link {
    pub fn new(url: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            url,
            parents: HashSet::new(),
            children: HashSet::new(),
            images: Vec::new(),
            titles: Vec::new(),
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct LinkGraph {
    links: HashMap<LinkId, Link>,
    link_ids: HashMap<String, LinkId>,
}

impl LinkGraph {
    // Update a link
    pub fn update(
        &mut self,
        url: &str,
        parent: &str,
        children: &[String],
        images: &[Image],
        titles: &[String],
    ) -> Result<()> {
        let maybe_parent = self.link_ids.get(parent).cloned();

        // for each child, add their id (if it exists) to this
        // links children
        let valid_children: Vec<LinkId> = children
            .iter()
            .filter_map(|c| self.link_ids.get(c).cloned())
            .collect();

        let link = self.force_get_link_id(url)?;

        if let Some(parent_id) = maybe_parent {
            link.parents.insert(parent_id);
        }

        link.children.extend(valid_children);

        let mut seen_images = HashSet::new();
        for img in images {
            if !seen_images.contains(&img.link) {
                seen_images.insert(img.link.clone());
                link.images.push(img.clone());
            }
        }

        let mut seen_titles = HashSet::new();
        for title in titles {
            if !seen_titles.contains(title) {
                seen_titles.insert(title.clone());
                link.titles.push(title.clone());
            }
        }

        let this_link_id = link.id;

        if let Some(parent_id) = maybe_parent {
            let parent_link = self
                .links
                .get_mut(&parent_id)
                .context("could not find parent link")?;
            parent_link.children.insert(this_link_id);
        }

        Ok(())
    }

    pub fn len(&self) -> usize {
        self.links.len()
    }

    pub fn link_visited(&self, url: &str) -> bool {
        self.link_ids.get(url).is_some()
    }

    /// This function will retrieve a valid link ID if the
    /// `url` is already contained within the links map.
    /// Otherwise, it will create a new Link with the
    /// given `url` and add it to the map, returning the
    /// new link ID.
    fn force_get_link_id(&mut self, url: &str) -> Result<&mut Link> {
        let this_link_id = if let Some(link_id) = self.link_ids.get(url) {
            *link_id
        } else {
            let new_link = Link::new(url.to_string());
            let new_link_id = new_link.id;

            // add new link to the map, return its id
            self.links
                .insert(new_link_id, new_link)
                .map_or(Ok(()), |_| Err(anyhow!("link already exists")))?;

            new_link_id
        };

        self.link_ids.insert(url.to_string(), this_link_id);
        self.links
            .get_mut(&this_link_id)
            .ok_or_else(|| anyhow!("failed to get link"))
    }

    // Get the ID for a link
}

impl<'a> IntoIterator for &'a LinkGraph {
    type Item = (&'a LinkId, &'a Link);
    type IntoIter = std::collections::hash_map::Iter<'a, LinkId, Link>;

    fn into_iter(self) -> Self::IntoIter {
        self.links.iter()
    }
}
