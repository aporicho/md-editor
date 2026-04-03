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

    /// 测试此标记是否与另一个标记相等（类型相同且属性相同）。
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
