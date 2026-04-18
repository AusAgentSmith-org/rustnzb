import { Injectable, signal } from '@angular/core';

export type WidthMode = 'compact' | 'expanded';

const STORAGE_KEY = 'rustnzb.widthMode';
const AUTO_BREAKPOINT = 1600;

@Injectable({ providedIn: 'root' })
export class WidthModeService {
  readonly mode = signal<WidthMode>('compact');

  constructor() {
    const saved = this.readSaved();
    this.apply(saved ?? this.pickAuto());
  }

  set(mode: WidthMode): void {
    localStorage.setItem(STORAGE_KEY, mode);
    this.apply(mode);
  }

  toggle(): void {
    this.set(this.mode() === 'compact' ? 'expanded' : 'compact');
  }

  private apply(mode: WidthMode): void {
    this.mode.set(mode);
    document.body.setAttribute('data-width-mode', mode);
  }

  private readSaved(): WidthMode | null {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === 'compact' || v === 'expanded' ? v : null;
  }

  private pickAuto(): WidthMode {
    return window.innerWidth >= AUTO_BREAKPOINT ? 'expanded' : 'compact';
  }
}
