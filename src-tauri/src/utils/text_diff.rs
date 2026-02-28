/// 计算文本增量更新
/// 返回：(需要回退的字符数, 需要新增的字符串)
pub fn compute_diff(old: &str, new: &str) -> (usize, String) {
    // 找到第一个不同的字符位置
    let common = old.chars()
        .zip(new.chars())
        .take_while(|(a, b)| a == b)
        .count();
    
    let backspace = old.chars().count() - common;
    let addition: String = new.chars().skip(common).collect();
    
    (backspace, addition)
}
