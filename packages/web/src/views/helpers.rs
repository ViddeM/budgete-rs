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

#[cfg(test)]
mod tests {
    use super::*;
    use api::models::CategorySpend;
    use rust_decimal::Decimal;
    use uuid::Uuid;

    fn cat(id: Uuid, name: &str, parent: Option<(Uuid, &str)>, total: &str) -> CategorySpend {
        CategorySpend {
            category_id: id,
            category_name: name.to_string(),
            category_color: "#aabbcc".to_string(),
            parent_id: parent.as_ref().map(|(pid, _)| *pid),
            parent_name: parent.as_ref().map(|(_, n)| n.to_string()),
            parent_color: parent.map(|_| "#112233".to_string()),
            total: total.parse().unwrap(),
        }
    }

    #[test]
    fn empty_input_returns_empty() {
        assert!(build_groups(&[]).is_empty());
    }

    #[test]
    fn top_level_only_each_becomes_own_group() {
        let food_id = Uuid::new_v4();
        let transport_id = Uuid::new_v4();
        let input = vec![
            cat(food_id, "Food", None, "300.00"),
            cat(transport_id, "Transport", None, "100.00"),
        ];
        let groups = build_groups(&input);
        assert_eq!(groups.len(), 2);
        // Sorted descending by total: Food (300) first.
        assert_eq!(groups[0].name, "Food");
        assert_eq!(groups[0].total, Decimal::from(300));
        assert!(groups[0].subcategories.is_empty());
        assert_eq!(groups[1].name, "Transport");
    }

    #[test]
    fn subcategories_merged_under_parent_with_summed_total() {
        let food_id = Uuid::new_v4();
        let groceries_id = Uuid::new_v4();
        let restaurant_id = Uuid::new_v4();
        let input = vec![
            cat(groceries_id, "Groceries", Some((food_id, "Food")), "200.00"),
            cat(restaurant_id, "Restaurant", Some((food_id, "Food")), "150.00"),
        ];
        let groups = build_groups(&input);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!(g.id, food_id);
        assert_eq!(g.name, "Food");
        assert_eq!(g.total, Decimal::from(350));
        assert_eq!(g.subcategories.len(), 2);
    }

    #[test]
    fn direct_parent_spend_plus_subcat_spend_accumulates() {
        // Parent category has its own transactions AND subcategory transactions.
        let food_id = Uuid::new_v4();
        let groceries_id = Uuid::new_v4();
        let input = vec![
            // Transactions assigned directly to "Food" (top-level, no parent_id)
            cat(food_id, "Food", None, "50.00"),
            // Transactions assigned to subcategory "Groceries" under "Food"
            cat(groceries_id, "Groceries", Some((food_id, "Food")), "200.00"),
        ];
        let groups = build_groups(&input);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].total, Decimal::from(250));
        // Only the subcategory is listed under subcategories (not the parent's own row).
        assert_eq!(groups[0].subcategories.len(), 1);
        assert_eq!(groups[0].subcategories[0].category_name, "Groceries");
    }

    #[test]
    fn sorted_descending_by_total() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        let input = vec![
            cat(a, "A", None, "100.00"),
            cat(b, "B", None, "500.00"),
            cat(c, "C", None, "250.00"),
        ];
        let groups = build_groups(&input);
        let totals: Vec<Decimal> = groups.iter().map(|g| g.total).collect();
        assert_eq!(
            totals,
            vec![Decimal::from(500), Decimal::from(250), Decimal::from(100)]
        );
    }
}
