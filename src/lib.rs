//! dbt (Data Build Tool) Model DAG Compiler for SigmaOS
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct DbtModel {
    pub name: String,
    pub raw_sql: String,
    pub dependencies: Vec<String>,
}

pub struct DbtCompiler;

impl DbtCompiler {
    pub fn parse_model(name: &str, raw_sql: &str) -> DbtModel {
        let mut dependencies = Vec::new();
        let mut cursor = raw_sql;

        while let Some(start) = cursor.find("{{ ref('") {
            let rest = &cursor[start + 8..];
            if let Some(end) = rest.find("') }}") {
                let dep_name = &rest[..end];
                if !dependencies.contains(&dep_name.to_string()) {
                    dependencies.push(dep_name.to_string());
                }
                cursor = &rest[end + 5..];
            } else {
                break;
            }
        }

        DbtModel {
            name: name.to_string(),
            raw_sql: raw_sql.to_string(),
            dependencies,
        }
    }

    pub fn compile_dag(models: &[DbtModel]) -> Result<Vec<String>, String> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        let all_names: HashSet<String> = models.iter().map(|m| m.name.clone()).collect();

        for m in models {
            in_degree.entry(m.name.clone()).or_insert(0);
            for dep in &m.dependencies {
                if !all_names.contains(dep) {
                    return Err(format!(
                        "Unresolved dependency '{}' in model '{}'",
                        dep, m.name
                    ));
                }
                adj.entry(dep.clone()).or_default().push(m.name.clone());
                *in_degree.entry(m.name.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: VecDeque<String> = VecDeque::new();
        for (name, &deg) in &in_degree {
            if deg == 0 {
                queue.push_back(name.clone());
            }
        }

        let mut sorted = Vec::new();
        while let Some(u) = queue.pop_front() {
            sorted.push(u.clone());
            if let Some(neighbors) = adj.get(&u) {
                for v in neighbors {
                    if let Some(deg) = in_degree.get_mut(v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != models.len() {
            return Err("Cyclic dependency detected in dbt models DAG".to_string());
        }

        Ok(sorted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbt_compiler_dag_topological_sort_aaa() {
        // Arrange
        let raw1 = "SELECT * FROM raw_data";
        let raw2 = "SELECT id, count FROM {{ ref('stg_users') }} WHERE active = 1";
        let raw3 = "SELECT * FROM {{ ref('dim_users') }} JOIN {{ ref('stg_users') }} USING (id)";

        let m1 = DbtCompiler::parse_model("stg_users", raw1);
        let m2 = DbtCompiler::parse_model("dim_users", raw2);
        let m3 = DbtCompiler::parse_model("fct_activity", raw3);

        // Act
        let order = DbtCompiler::compile_dag(&[m3, m1, m2]).expect("DAG compile failed");

        // Assert
        assert_eq!(order, vec!["stg_users", "dim_users", "fct_activity"]);
    }
}
