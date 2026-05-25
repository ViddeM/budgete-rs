use api::models::CategorySpend;
use rust_decimal::Decimal;
use std::cmp::Reverse;
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Category grouping
// ---------------------------------------------------------------------------

/// A top-level category with its subcategories and combined total.
pub struct CategoryGroup {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub total: Decimal,
    pub subcategories: Vec<CategorySpend>,
}

/// Group a flat `CategorySpend` list by top-level category, summing
/// subcategory totals under their parent. Returns groups sorted by total
/// descending (largest spend first).
pub fn build_groups(cats: &[CategorySpend]) -> Vec<CategoryGroup> {
    let mut map: HashMap<Uuid, CategoryGroup> = HashMap::new();

    for cat in cats {
        let (gid, gname, gcolor) = if let Some(pid) = cat.parent_id {
            (
                pid,
                cat.parent_name.clone().unwrap_or_default(),
                cat.parent_color
                    .clone()
                    .unwrap_or_else(|| cat.category_color.clone()),
            )
        } else {
            (
                cat.category_id,
                cat.category_name.clone(),
                cat.category_color.clone(),
            )
        };

        let entry = map.entry(gid).or_insert_with(|| CategoryGroup {
            id: gid,
            name: gname,
            color: gcolor,
            total: Decimal::ZERO,
            subcategories: vec![],
        });

        entry.total += cat.total;

        if cat.parent_id.is_some() {
            entry.subcategories.push(cat.clone());
        }
    }

    let mut groups: Vec<CategoryGroup> = map.into_values().collect();
    groups.sort_by_key(|g| Reverse(g.total));
    groups
}
