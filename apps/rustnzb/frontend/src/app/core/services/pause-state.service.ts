import { Injectable, signal } from '@angular/core';

/** Shared, authoritative UI view of the backend's global pause state. */
@Injectable({ providedIn: 'root' })
export class PauseStateService {
  readonly paused = signal(false);
}
