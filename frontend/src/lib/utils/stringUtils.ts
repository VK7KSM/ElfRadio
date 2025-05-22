/**
 * Truncates a string in the middle, preserving a specified number of
 * characters at the beginning and end, and inserting an ellipsis.
 * Only truncates if the string's length exceeds the sum of headChars and tailChars.
 *
 * @param str The string to truncate.
 * @param headChars The number of characters to keep at the beginning.
 * @param tailChars The number of characters to keep at the end.
 * @returns The truncated string or the original string if no truncation is needed.
 */
export function truncateStringMiddle(str: string, headChars: number, tailChars: number): string {
  if (typeof str !== 'string') {
    console.error('truncateStringMiddle: input is not a string', str);
    return ''; // Or handle error appropriately
  }
  // Ensure headChars and tailChars are non-negative integers
  const hc = Math.max(0, Math.floor(headChars));
  const tc = Math.max(0, Math.floor(tailChars));

  const minLengthForTruncation = hc + tc + 1; // Must be at least 1 char longer than preserved chars to insert "..."

  if (str.length <= minLengthForTruncation || str.length <= hc + tc) { // If string is too short to meaningfully truncate or just fits
    return str; // No truncation needed
  }
  
  const head = str.substring(0, hc);
  const tail = str.substring(str.length - tc);
  return `${head}...${tail}`;
}

/**
 * 将 UUID 字符串截断为固定的短格式。
 * 例如："b187148a-2fd8-4cd8-a5cc-61c1806bb97d" 将变为 "b1...6bb97d"。
 * 
 * @param uuid 完整的 UUID 字符串。
 * @returns 截断后的 UUID 字符串，或者在输入不符合预期长度时返回原始字符串。
 */
export const truncateUuidToFixedShortFormat = (uuid: string): string => {
  if (typeof uuid !== 'string') {
    // 如果输入不是字符串，则返回空字符串或采取其他错误处理
    return ''; 
  }

  // UUID 的标准长度通常是 36 个字符
  // 我们期望截取前2位和后6位，总共需要至少8位。
  if (uuid.length < 8) {
    // 如果字符串太短，无法按预期截断，则返回原始字符串
    return uuid;
  }

  const firstPart = uuid.substring(0, 2);
  const lastPart = uuid.substring(uuid.length - 6);

  return `${firstPart}...${lastPart}`;
}; 