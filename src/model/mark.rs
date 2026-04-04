use std::sync::Arc;

use super::Attrs;
use super::schema::MarkType;

/// 标记——叠加在文本节点上的行内样式（粗体、斜体、链接等）。
///
/// 标记持有一个类型（MarkType）和一组属性（如链接的 href）。
/// 标记是不可变的。
///
/// 对应 prosemirror-model/src/mark.ts
#[derive(Debug, Clone)]
pub struct Mark {
    /// 标记类型
    pub mark_type: Arc<MarkType>,
    /// 标记属性
    pub attrs: Attrs,
}

impl Mark {
    /// 创建一个新标记。
    pub fn new(mark_type: Arc<MarkType>, attrs: Attrs) -> Self {
        Self { mark_type, attrs }
    }

    /// 标记相等：类型指针相同且属性相同（ptr_eq 比较，MarkType Arc 在 Schema 中保持一致）。
    pub fn eq(&self, other: &Mark) -> bool {
        Arc::ptr_eq(&self.mark_type, &other.mark_type) && self.attrs == other.attrs
    }

    /// 将此标记加入标记集，返回新集合。
    /// 按 rank 排序插入，处理排斥关系。
    /// 如果此标记已在集合中，返回原集合的克隆。
    pub fn add_to_set(&self, set: &[Mark]) -> Vec<Mark> {
        let mut copy: Option<Vec<Mark>> = None;
        let mut placed = false;

        for i in 0..set.len() {
            let other = &set[i];

            // 已经在集合中
            if self.eq(other) {
                return set.to_vec();
            }

            // 此标记排斥 other → 跳过 other
            if self.mark_type.excludes(&other.mark_type) {
                if copy.is_none() {
                    copy = Some(set[..i].to_vec());
                }
            }
            // other 排斥此标记 → 无法加入，返回原集合
            else if other.mark_type.excludes(&self.mark_type) {
                return set.to_vec();
            }
            // 正常情况
            else {
                // 按 rank 找到插入位置
                if !placed && other.mark_type.rank > self.mark_type.rank {
                    if copy.is_none() {
                        copy = Some(set[..i].to_vec());
                    }
                    copy.as_mut().unwrap().push(self.clone());
                    placed = true;
                }
                if let Some(ref mut c) = copy {
                    c.push(other.clone());
                }
            }
        }

        let mut result = copy.unwrap_or_else(|| set.to_vec());
        if !placed {
            result.push(self.clone());
        }
        result
    }

    /// 从标记集中移除此标记，返回新集合。
    /// 如果此标记不在集合中，返回原集合的克隆。
    pub fn remove_from_set(&self, set: &[Mark]) -> Vec<Mark> {
        for i in 0..set.len() {
            if self.eq(&set[i]) {
                let mut result = set[..i].to_vec();
                result.extend_from_slice(&set[i + 1..]);
                return result;
            }
        }
        set.to_vec()
    }

    /// 检查此标记是否在给定的标记集中。
    pub fn is_in_set(&self, set: &[Mark]) -> bool {
        set.iter().any(|m| self.eq(m))
    }

    /// 测试两个标记集是否完全相同。
    pub fn same_set(a: &[Mark], b: &[Mark]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        a.iter().zip(b.iter()).all(|(ma, mb)| ma.eq(mb))
    }

    /// 从标记切片创建按 rank 排序的标记集。
    pub fn set_from(marks: &[Mark]) -> Vec<Mark> {
        if marks.is_empty() {
            return Vec::new();
        }
        if marks.len() == 1 {
            return marks.to_vec();
        }
        let mut sorted = marks.to_vec();
        sorted.sort_by_key(|m| m.mark_type.rank);
        sorted
    }

    /// 空标记集。
    pub fn none() -> Vec<Mark> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::schema::MarkType;
    use std::collections::BTreeMap;
    use std::sync::Arc;

    fn mt(name: &str, rank: usize) -> Arc<MarkType> {
        Arc::new(MarkType { name: name.into(), rank, excluded: vec![], inclusive: None })
    }

    fn mk(mark_type: Arc<MarkType>) -> Mark {
        Mark { mark_type, attrs: BTreeMap::new() }
    }

    #[test]
    fn add_to_set_idempotent() {
        let m = mk(mt("bold", 0));
        let set = vec![m.clone()];
        let result = m.add_to_set(&set);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn add_to_set_rank_order() {
        let m_em = mk(mt("em", 10));
        let m_bold = mk(mt("bold", 5));
        let set = vec![m_em.clone()];
        let result = m_bold.add_to_set(&set);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].mark_type.name, "bold");
        assert_eq!(result[1].mark_type.name, "em");
    }

    #[test]
    fn add_to_set_this_excludes_other() {
        let mt_b = Arc::new(MarkType { name: "b".into(), rank: 1, excluded: vec![], inclusive: None });
        let mt_a = Arc::new(MarkType {
            name: "a".into(), rank: 0,
            excluded: vec![Arc::clone(&mt_b)],
            inclusive: None,
        });
        let set = vec![mk(Arc::clone(&mt_b))];
        let result = mk(Arc::clone(&mt_a)).add_to_set(&set);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].mark_type.name, "a");
    }

    #[test]
    fn add_to_set_other_excludes_this() {
        let mt_a = Arc::new(MarkType { name: "a".into(), rank: 0, excluded: vec![], inclusive: None });
        let mt_b = Arc::new(MarkType {
            name: "b".into(), rank: 1,
            excluded: vec![Arc::clone(&mt_a)],
            inclusive: None,
        });
        let set = vec![mk(Arc::clone(&mt_b))];
        let result = mk(Arc::clone(&mt_a)).add_to_set(&set);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].mark_type.name, "b");
    }

    #[test]
    fn remove_from_set_exists() {
        let m = mk(mt("bold", 0));
        let result = m.remove_from_set(&[m.clone()]);
        assert!(result.is_empty());
    }

    #[test]
    fn remove_from_set_not_exists() {
        let m1 = mk(mt("bold", 0));
        let m2 = mk(mt("em", 1));
        let result = m2.remove_from_set(&[m1.clone()]);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn same_set_length_diff() {
        let m = mk(mt("bold", 0));
        assert!(!Mark::same_set(&[m.clone()], &[]));
    }

    #[test]
    fn same_set_identical() {
        let m = mk(mt("bold", 0));
        assert!(Mark::same_set(&[m.clone()], &[m.clone()]));
    }

    #[test]
    fn same_set_content_diff() {
        let m1 = mk(mt("bold", 0));
        let m2 = mk(mt("em", 1));
        assert!(!Mark::same_set(&[m1], &[m2]));
    }

    #[test]
    fn set_from_sorted() {
        let m_em = mk(mt("em", 10));
        let m_bold = mk(mt("bold", 5));
        let result = Mark::set_from(&[m_em, m_bold]);
        assert_eq!(result[0].mark_type.name, "bold");
        assert_eq!(result[1].mark_type.name, "em");
    }

    #[test]
    fn set_from_empty() {
        assert!(Mark::set_from(&[]).is_empty());
    }

    #[test]
    fn is_in_set() {
        let m = mk(mt("bold", 0));
        assert!(m.is_in_set(&[m.clone()]));
        assert!(!m.is_in_set(&[]));
    }
}
