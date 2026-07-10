import '@angular/compiler';

import { beforeEach, describe, expect, it, vi } from 'vitest';

import { AddNzbService } from './add-nzb.service';
import { WidthModeService } from './width-mode.service';

describe('WidthModeService', () => {
  beforeEach(() => {
    localStorage.clear();
    document.body.removeAttribute('data-width-mode');
  });

  it.each([
    [1599, 'compact'],
    [1600, 'expanded'],
  ] as const)('selects the automatic mode at viewport width %i', (width, expected) => {
    vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(width);
    const service = new WidthModeService();
    expect(service.mode()).toBe(expected);
    expect(document.body.getAttribute('data-width-mode')).toBe(expected);
  });

  it('prefers a valid saved mode over the automatic mode', () => {
    localStorage.setItem('rustnzb.widthMode', 'compact');
    vi.spyOn(window, 'innerWidth', 'get').mockReturnValue(2000);
    expect(new WidthModeService().mode()).toBe('compact');
  });

  it('persists, applies, and toggles explicit selections', () => {
    const service = new WidthModeService();
    service.set('expanded');
    expect(localStorage.getItem('rustnzb.widthMode')).toBe('expanded');
    expect(document.body.getAttribute('data-width-mode')).toBe('expanded');
    service.toggle();
    expect(service.mode()).toBe('compact');
  });
});

describe('AddNzbService', () => {
  it('emits exactly once for every panel toggle request', () => {
    const service = new AddNzbService();
    const observer = vi.fn();
    service.panelToggle$.subscribe(observer);
    service.togglePanel();
    service.togglePanel();
    expect(observer).toHaveBeenCalledTimes(2);
  });
});
