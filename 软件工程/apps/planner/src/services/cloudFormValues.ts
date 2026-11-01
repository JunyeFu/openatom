/** Form values use local calendar time / seconds; protocol values remain UTC / integer ms. */
export function millisecondsToSeconds(ms: number): string {
  const value = BigInt(ms), fraction = String(value % 1000n).padStart(3, '0').replace(/0+$/, '');
  return `${value / 1000n}${fraction ? '.' + fraction : ''}`;
}

/** Execution query windows exclude their end; include the last covered integer millisecond. */
export function executionQueryEnd(coveredMs: number): number {
  return Math.min(Number.MAX_SAFE_INTEGER, coveredMs + 1);
}

export function secondsToMilliseconds(raw: string, label: string, positive = false): number {
  if (!/^\d+(?:\.\d{1,3})?$/.test(raw)) throw new Error(`${label}请输入秒数，最多保留三位小数。`);
  const [whole, fraction = ''] = raw.split('.');
  const value = BigInt(whole) * 1000n + BigInt(fraction.padEnd(3, '0'));
  if (value > BigInt(Number.MAX_SAFE_INTEGER) || (positive && value === 0n)) throw new Error(`${label}需大于 0 且在可用时间范围内。`);
  return Number(value);
}

const pad = (value: number, length = 2) => String(value).padStart(length, '0');
function localInput(date: Date) {
  return `${pad(date.getFullYear(), 4)}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}T${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}.${pad(date.getMilliseconds(), 3)}`;
}

/** Calendar controls cannot represent microseconds. This never changes the original UTC draft. */
export function utcToLocalInput(utc: string): string {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{1,6})?Z$/.test(utc)) return '';
  const date = new Date(utc);
  return Number.isNaN(date.valueOf()) ? '' : localInput(date);
}

export function localInputToUtc(raw: string): string {
  if (!raw) return '';
  const parts = /^(\d{4}-\d{2}-\d{2}T\d{2}:\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?$/.exec(raw);
  if (!parts) throw new Error('请选择有效的本地日期和时间。');
  const date = new Date(raw);
  const expected = `${parts[1]}:${parts[2] ?? '00'}.${(parts[3] ?? '').padEnd(3, '0')}`;
  if (Number.isNaN(date.valueOf()) || localInput(date) !== expected) throw new Error('这个日期时间不存在，请重新选择。');
  return date.toISOString();
}

export const localTimeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
export function cloudTimeLabel(utc: string): string {
  const fraction = /\.(\d+)(?:Z|[+-]\d{2}:\d{2})$/.exec(utc)?.[1];
  return new Date(utc).toLocaleString('zh-CN', { year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit', second: '2-digit', hourCycle: 'h23' }) + (fraction ? '.' + fraction : '');
}
