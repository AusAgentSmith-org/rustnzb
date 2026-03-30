import { Component, OnInit, OnDestroy, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { MatProgressBarModule } from '@angular/material/progress-bar';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { ApiService } from '../../core/services/api.service';
import { NzbJob, QueueResponse } from '../../core/models/queue.model';

@Component({
  selector: 'app-queue-view',
  standalone: true,
  imports: [CommonModule, MatIconModule, MatButtonModule, MatProgressBarModule, MatSnackBarModule],
  template: `
    <div class="queue-toolbar">
      <span class="queue-stats">{{ jobs().length }} items · {{ formatBytes(remainingBytes()) }} remaining</span>
    </div>

    <div class="job-list">
      @for (job of jobs(); track job.id) {
        <div class="job-item">
          <div class="job-icon">{{ statusIcon(job.status) }}</div>
          <div class="job-info">
            <div class="job-name">{{ job.name }}</div>
            <div class="job-meta">
              <span class="tag" [class]="'tag-' + job.status">{{ job.status | uppercase }}</span>
              <span>{{ formatBytes(job.total_bytes) }}</span>
              <span>{{ priorityLabel(job.priority) }}</span>
              @if (job.category) { <span>{{ job.category }}</span> }
              @if (job.status === 'downloading' && job.speed_bps > 0) {
                <span>ETA {{ eta(job) }}</span>
              }
            </div>
          </div>
          @if (job.status === 'downloading' && job.speed_bps > 0) {
            <div class="job-speed">{{ formatSpeed(job.speed_bps) }}</div>
          }
          <div class="job-progress">
            <div class="progress-bar">
              <div class="progress-fill" [class]="progressClass(job.status)"
                   [style.width.%]="percent(job)"></div>
            </div>
            <div class="progress-text">
              @if (job.status === 'downloading') {
                {{ percent(job) }}% · {{ formatBytes(job.downloaded_bytes) }} / {{ formatBytes(job.total_bytes) }}
              } @else if (job.status === 'queued') {
                Waiting...
              } @else if (job.status === 'paused') {
                {{ percent(job) }}% · Paused
              } @else {
                {{ job.status }}
              }
            </div>
          </div>
          <div class="job-actions">
            @if (job.status === 'downloading' || job.status === 'queued') {
              <button class="action-btn" (click)="pauseJob(job.id)" title="Pause">⏸</button>
            }
            @if (job.status === 'paused') {
              <button class="action-btn" (click)="resumeJob(job.id)" title="Resume">▶</button>
            }
            <button class="action-btn" (click)="deleteJob(job.id)" title="Remove">✕</button>
          </div>
        </div>
      }

      @if (jobs().length === 0) {
        <div class="empty-state">
          <div class="empty-icon">📥</div>
          <p>No downloads in queue</p>
          <p class="hint">Add an NZB file or browse newsgroups to get started</p>
        </div>
      }
    </div>
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .queue-toolbar {
      display: flex; align-items: center; padding: 10px 16px;
      background: #0d1117; border-bottom: 1px solid #21262d; font-size: 12px; color: #8b949e;
    }
    .job-list { flex: 1; overflow-y: auto; }
    .job-item {
      display: flex; align-items: center; gap: 12px;
      padding: 12px 16px; border-bottom: 1px solid #21262d;
    }
    .job-item:hover { background: #161b22; }
    .job-icon { font-size: 20px; width: 24px; text-align: center; }
    .job-info { flex: 1; min-width: 0; }
    .job-name { font-weight: 600; font-size: 14px; margin-bottom: 4px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .job-meta { display: flex; gap: 12px; font-size: 11px; color: #8b949e; flex-wrap: wrap; }
    .tag { padding: 1px 6px; border-radius: 3px; font-size: 10px; font-weight: 600; }
    .tag-downloading { background: #0d419d; color: #58a6ff; }
    .tag-queued { background: #1c2128; color: #8b949e; }
    .tag-paused { background: #3d1d00; color: #d29922; }
    .tag-verifying, .tag-repairing, .tag-extracting, .tag-post_processing { background: #1a3a1a; color: #3fb950; }
    .tag-completed { background: #1a3a1a; color: #3fb950; }
    .tag-failed { background: #3d1418; color: #f85149; }
    .job-speed { font-family: monospace; font-size: 12px; color: #58a6ff; width: 80px; text-align: right; }
    .job-progress { width: 200px; }
    .progress-bar { height: 6px; background: #21262d; border-radius: 3px; overflow: hidden; margin-bottom: 4px; }
    .progress-fill { height: 100%; border-radius: 3px; transition: width 0.3s; }
    .progress-fill.blue { background: linear-gradient(90deg, #1f6feb, #58a6ff); }
    .progress-fill.green { background: #3fb950; }
    .progress-fill.yellow { background: #d29922; }
    .progress-text { font-size: 11px; color: #8b949e; text-align: right; }
    .job-actions { display: flex; gap: 4px; }
    .action-btn {
      padding: 4px 8px; border-radius: 4px; border: 1px solid #30363d;
      background: #21262d; color: #c9d1d9; cursor: pointer; font-size: 12px;
    }
    .action-btn:hover { background: #30363d; }
    .empty-state { text-align: center; padding: 64px 16px; color: #484f58; }
    .empty-icon { font-size: 48px; margin-bottom: 16px; }
    .hint { font-size: 12px; margin-top: 8px; }
  `],
})
export class QueueViewComponent implements OnInit, OnDestroy {
  jobs = signal<NzbJob[]>([]);
  remainingBytes = signal(0);
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  constructor(private api: ApiService, private snackBar: MatSnackBar) {}

  ngOnInit(): void {
    this.loadQueue();
    this.pollTimer = setInterval(() => this.loadQueue(), 2000);
  }

  ngOnDestroy(): void {
    if (this.pollTimer) clearInterval(this.pollTimer);
  }

  loadQueue(): void {
    this.api.get<QueueResponse>('/queue').subscribe({
      next: (r) => {
        this.jobs.set(r.jobs);
        this.remainingBytes.set(r.jobs.reduce((sum, j) => sum + (j.total_bytes - j.downloaded_bytes), 0));
      },
      error: () => {},
    });
  }

  pauseJob(id: string): void {
    this.api.post(`/queue/${id}/pause`).subscribe(() => this.loadQueue());
  }

  resumeJob(id: string): void {
    this.api.post(`/queue/${id}/resume`).subscribe(() => this.loadQueue());
  }

  deleteJob(id: string): void {
    this.api.delete(`/queue/${id}`).subscribe(() => {
      this.loadQueue();
      this.snackBar.open('Removed from queue', 'Close', { duration: 2000 });
    });
  }

  percent(job: NzbJob): number {
    if (job.total_bytes === 0) return 0;
    return Math.round((job.downloaded_bytes / job.total_bytes) * 100);
  }

  eta(job: NzbJob): string {
    if (job.speed_bps === 0) return '∞';
    const remaining = job.total_bytes - job.downloaded_bytes;
    const secs = remaining / job.speed_bps;
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`;
  }

  statusIcon(status: string): string {
    const icons: Record<string, string> = {
      downloading: '📥', queued: '⏳', paused: '⏸', verifying: '🔍',
      repairing: '🔧', extracting: '📦', completed: '✅', failed: '❌',
    };
    return icons[status] || '⏳';
  }

  priorityLabel(p: number): string {
    return ['Low', 'Normal', 'High', 'Force'][p] || 'Normal';
  }

  progressClass(status: string): string {
    if (status === 'paused') return 'yellow';
    if (['verifying', 'repairing', 'extracting', 'completed'].includes(status)) return 'green';
    return 'blue';
  }

  formatSpeed(bps: number): string {
    if (bps === 0) return '';
    const k = 1024;
    const sizes = ['B/s', 'KB/s', 'MB/s', 'GB/s'];
    const i = Math.floor(Math.log(bps) / Math.log(k));
    return parseFloat((bps / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
}
