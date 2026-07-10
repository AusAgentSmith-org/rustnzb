import '@angular/compiler';

import { describe, expect, it, vi } from 'vitest';
import { Observable, of } from 'rxjs';

import { ApiService } from '../../core/services/api.service';
import { LogsViewComponent } from './logs-view.component';

type TestLog = { seq: number; level: string; message: string; timestamp: string; target?: string };

function setup() {
  const api = {
    get: vi.fn<(...args: unknown[]) => Observable<{ entries: TestLog[] }>>(() =>
      of({ entries: [] }),
    ),
  };
  const component = new LogsViewComponent(api as unknown as ApiService);
  component.entries.set([
    { seq: 1, level: 'INFO', message: 'server started', timestamp: '2026-07-10T10:11:12.123Z', target: 'app' },
    { seq: 2, level: 'ERROR', message: 'CRC mismatch', timestamp: '2026-07-10T10:11:13Z', target: 'decode' },
  ]);
  return { api, component };
}

describe('LogsViewComponent', () => {
  it('reactively combines level and regex filters', () => {
    const { component } = setup();
    component.levelFilter.set('ERROR');
    expect(component.visibleEntries().map((entry) => entry.seq)).toEqual([2]);
    component.filter.set('does-not-match');
    expect(component.visibleEntries()).toEqual([]);
  });

  it('falls back to a literal match for invalid regular expressions', () => {
    const { component } = setup();
    component.entries.update((entries) => [
      ...entries,
      { seq: 3, level: 'WARN', message: 'literal [ value', timestamp: '', target: '' },
    ]);
    component.filter.set('[');
    expect(component.visibleEntries().map((entry) => entry.seq)).toEqual([3]);
  });

  it('appends incremental logs and requests subsequent sequence numbers', () => {
    const { api, component } = setup();
    api.get
      .mockReturnValueOnce(of({ entries: [{ seq: 4, level: 'INFO', message: 'new', timestamp: '' }] }))
      .mockReturnValueOnce(of({ entries: [] }));
    component.entries.set([]);
    component.loadLogs();
    component.loadLogs();
    expect(api.get).toHaveBeenNthCalledWith(1, '/logs', {});
    expect(api.get).toHaveBeenNthCalledWith(2, '/logs', { after_seq: '4' });
  });

  it('maps timestamp and level display formats', () => {
    const { component } = setup();
    expect(component.formatTs('2026-07-10T10:11:12.123456Z')).toBe('10:11:12.123');
    expect(component.levelClass('ERROR')).toBe('err');
    expect(component.levelClass('warning')).toBe('warn');
    expect(component.levelClass('TRACE')).toBe('dbg');
  });

  it('clears buffered entries and toggles following state', () => {
    const { component } = setup();
    component.toggleFollow();
    expect(component.follow()).toBe(false);
    component.clear();
    expect(component.entries()).toEqual([]);
  });
});
