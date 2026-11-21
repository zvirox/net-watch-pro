import { describe, expect, it } from 'vitest';
import { formatBytes } from './format';

describe('formatBytes', () => {
  it('formats zero correctly', () => {
    expect(formatBytes(0)).toBe('0 B');
  });

  it('formats byte values under 1024 B', () => {
    expect(formatBytes(1)).toBe('1 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(1023)).toBe('1023 B');
  });

  it('formats KB, MB, GB, and TB values', () => {
    expect(formatBytes(1024)).toBe('1.0 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(1024 * 1024)).toBe('1.0 MB');
    expect(formatBytes(50 * 1024 * 1024)).toBe('50.0 MB');
    expect(formatBytes(150 * 1024 * 1024)).toBe('150 MB');
    expect(formatBytes(1024 * 1024 * 1024)).toBe('1.0 GB');
    expect(formatBytes(5 * 1024 * 1024 * 1024 * 1024)).toBe('5.0 TB');
  });

  it('formats huge numbers up to TB without overflowing units', () => {
    const petabyte = 1024 * 1024 * 1024 * 1024 * 1024;
    expect(formatBytes(petabyte)).toBe('1024 MB'.replace('MB', 'TB'));
  });

  it('neutralizes negative values to em-dash (6d05693 negative unattributed regression)', () => {
    expect(formatBytes(-1)).toBe('—');
    expect(formatBytes(-1000)).toBe('—');
    expect(formatBytes(-1024 * 1024)).toBe('—');
  });

  it('handles NaN and non-finite values safely', () => {
    expect(formatBytes(NaN)).toBe('—');
    expect(formatBytes(Infinity)).toBe('—');
    expect(formatBytes(-Infinity)).toBe('—');
  });
});
