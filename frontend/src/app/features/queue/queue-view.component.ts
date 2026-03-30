import { Component, OnInit, OnDestroy, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { HttpClient } from '@angular/common/http';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonModule } from '@angular/material/button';
import { MatProgressBarModule } from '@angular/material/progress-bar';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { ApiService } from '../../core/services/api.service';
import {
  NzbJob, QueueResponse, HistoryEntry, StageResult,
  ServerArticleStats, LogEntry, LogsResponse,
} from '../../core/models/queue.model';

interface CategoryConfig {
  name: string;
  output_dir: string | null;
  post_processing: number;
}

@Component({
  selector: 'app-queue-view',
  standalone: true,
  imports: [CommonModule, FormsModule, MatIconModule, MatButtonModule, MatProgressBarModule, MatSnackBarModule],
  template: `
    <div class="queue-toolbar">
      <span class="queue-stats">{{ jobs().length }} active · {{ formatBytes(remainingBytes()) }} remaining</span>
      <button class="toolbar-btn" (click)="showAddPanel = !showAddPanel">
        @if (showAddPanel) { Hide Add NZB } @else { Add NZB }
      </button>
    </div>

    @if (showAddPanel) {
      <div class="add-panel">
        <div class="add-tabs">
          <button class="tab-btn" [class.active]="addMode === 'file'" (click)="addMode = 'file'">Upload File</button>
          <button class="tab-btn" [class.active]="addMode === 'url'" (click)="addMode = 'url'">From URL</button>
        </div>
        @if (addMode === 'file') {
          <div class="add-form">
            <div class="form-row">
              <input type="file" accept=".nzb" class="file-input" (change)="onFileSelected($event)" />
            </div>
            <div class="form-row">
              <label class="form-label">Category</label>
              <select class="form-select" [(ngModel)]="addCategory">
                <option value="">None</option>
                @for (cat of categories(); track cat.name) { <option [value]="cat.name">{{ cat.name }}</option> }
              </select>
              <label class="form-label">Priority</label>
              <select class="form-select" [(ngModel)]="addPriority">
                <option [ngValue]="0">Low</option><option [ngValue]="1">Normal</option>
                <option [ngValue]="2">High</option><option [ngValue]="3">Force</option>
              </select>
            </div>
            <div class="form-row">
              <button class="submit-btn" [disabled]="!selectedFile || uploading" (click)="uploadFile()">
                @if (uploading) { Uploading... } @else { Upload }
              </button>
            </div>
          </div>
        }
        @if (addMode === 'url') {
          <div class="add-form">
            <div class="form-row">
              <input type="text" class="form-input" placeholder="https://example.com/file.nzb" [(ngModel)]="addUrl" />
            </div>
            <div class="form-row">
              <label class="form-label">Category</label>
              <select class="form-select" [(ngModel)]="addCategory">
                <option value="">None</option>
                @for (cat of categories(); track cat.name) { <option [value]="cat.name">{{ cat.name }}</option> }
              </select>
              <label class="form-label">Priority</label>
              <select class="form-select" [(ngModel)]="addPriority">
                <option [ngValue]="0">Low</option><option [ngValue]="1">Normal</option>
                <option [ngValue]="2">High</option><option [ngValue]="3">Force</option>
              </select>
            </div>
            <div class="form-row">
              <button class="submit-btn" [disabled]="!addUrl || uploading" (click)="addFromUrl()">
                @if (uploading) { Adding... } @else { Add }
              </button>
            </div>
          </div>
        }
      </div>
    }

    <!-- Active queue -->
    <div class="section-list" [class.has-history]="history().length > 0">
      @for (job of jobs(); track job.id) {
        <div class="job-item" [class.selected]="selectedId() === job.id" (click)="selectJob(job.id)">
          <div class="job-icon">{{ statusIcon(job.status) }}</div>
          <div class="job-info">
            <div class="job-name">{{ job.name }}</div>
            <div class="job-meta">
              <span class="tag" [class]="'tag-' + job.status">{{ job.status | uppercase }}</span>
              <span>{{ formatBytes(job.total_bytes) }}</span>
              <select class="inline-select" [ngModel]="job.priority" (ngModelChange)="changeJobPriority(job.id, $event)" (click)="$event.stopPropagation()">
                <option [ngValue]="0">Low</option><option [ngValue]="1">Normal</option>
                <option [ngValue]="2">High</option><option [ngValue]="3">Force</option>
              </select>
              <select class="inline-select" [ngModel]="job.category" (ngModelChange)="changeJobCategory(job.id, $event)" (click)="$event.stopPropagation()">
                <option value="">None</option>
                @for (cat of categories(); track cat.name) { <option [value]="cat.name">{{ cat.name }}</option> }
              </select>
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
              <div class="progress-fill" [class]="progressClass(job.status)" [style.width.%]="percent(job)"></div>
            </div>
            <div class="progress-text">
              @if (job.status === 'downloading') {
                {{ percent(job) }}% · {{ formatBytes(job.downloaded_bytes) }} / {{ formatBytes(job.total_bytes) }}
              } @else if (job.status === 'queued') { Waiting...
              } @else if (job.status === 'paused') { {{ percent(job) }}% · Paused
              } @else { {{ job.status }} }
            </div>
          </div>
          <div class="job-actions">
            @if (job.status === 'downloading' || job.status === 'queued') {
              <button class="action-btn" (click)="pauseJob(job.id); $event.stopPropagation()" title="Pause">||</button>
            }
            @if (job.status === 'paused') {
              <button class="action-btn" (click)="resumeJob(job.id); $event.stopPropagation()" title="Resume">></button>
            }
            <button class="action-btn" (click)="deleteJob(job.id); $event.stopPropagation()" title="Remove">x</button>
          </div>
        </div>

        <!-- Detail panel for active job -->
        @if (selectedId() === job.id) {
          <div class="detail-panel">
            <div class="detail-tabs">
              <button class="dtab" [class.active]="detailTab() === 'info'" (click)="detailTab.set('info')">Info</button>
              <button class="dtab" [class.active]="detailTab() === 'logs'" (click)="detailTab.set('logs'); loadJobLogs(job.id)">Logs</button>
            </div>
            @if (detailTab() === 'info') {
              <div class="detail-section">
                <div class="detail-grid">
                  <div class="dg-label">Status</div><div class="dg-value">{{ job.status | uppercase }}</div>
                  <div class="dg-label">Size</div><div class="dg-value">{{ formatBytes(job.total_bytes) }}</div>
                  <div class="dg-label">Downloaded</div><div class="dg-value">{{ formatBytes(job.downloaded_bytes) }} ({{ percent(job) }}%)</div>
                  <div class="dg-label">Files</div><div class="dg-value">{{ job.files_completed }} / {{ job.file_count }}</div>
                  <div class="dg-label">Articles</div><div class="dg-value">{{ job.articles_downloaded }} / {{ job.article_count }} ({{ job.articles_failed }} failed)</div>
                  <div class="dg-label">Added</div><div class="dg-value">{{ formatDate(job.added_at) }}</div>
                  <div class="dg-label">Category</div><div class="dg-value">{{ job.category || 'None' }}</div>
                  <div class="dg-label">Priority</div><div class="dg-value">{{ priorityLabel(job.priority) }}</div>
                  @if (job.speed_bps > 0) {
                    <div class="dg-label">Speed</div><div class="dg-value">{{ formatSpeed(job.speed_bps) }}</div>
                  }
                  @if (job.error_message) {
                    <div class="dg-label">Error</div><div class="dg-value error-msg">{{ job.error_message }}</div>
                  }
                </div>
                @if (job.server_stats && job.server_stats.length > 0) {
                  <div class="detail-sub">Server Stats</div>
                  <div class="server-stats">
                    @for (ss of job.server_stats; track ss.server_id) {
                      <div class="ss-row">
                        <span class="ss-name">{{ ss.server_name }}</span>
                        <span>{{ ss.articles_downloaded }} articles</span>
                        <span>{{ formatBytes(ss.bytes_downloaded) }}</span>
                        @if (ss.articles_failed > 0) { <span class="error-msg">{{ ss.articles_failed }} failed</span> }
                      </div>
                    }
                  </div>
                }
              </div>
            }
            @if (detailTab() === 'logs') {
              <div class="log-viewer">
                @if (jobLogs().loading) { <div class="log-loading">Loading logs...</div>
                } @else if (jobLogs().entries.length === 0) { <div class="log-empty">No log entries</div>
                } @else {
                  @for (log of jobLogs().entries; track log.seq) {
                    <div class="log-line" [class]="'log-' + log.level.toLowerCase()">
                      <span class="log-ts">{{ log.timestamp | slice:11:19 }}</span>
                      <span class="log-msg">{{ log.message }}</span>
                    </div>
                  }
                }
              </div>
            }
          </div>
        }
      }

      @if (jobs().length === 0 && history().length === 0) {
        <div class="empty-state">
          <div class="empty-icon">No downloads</div>
          <p class="hint">Add an NZB file to get started</p>
        </div>
      }
    </div>

    <!-- History section -->
    @if (history().length > 0) {
      <div class="history-header">
        <span class="history-title">History ({{ history().length }})</span>
        <span class="spacer"></span>
        <button class="toolbar-btn warn" (click)="clearHistory()">Clear All</button>
      </div>
      <div class="section-list history-list">
        @for (e of history(); track e.id) {
          <div class="job-item history-item" [class.selected]="selectedId() === e.id" (click)="selectHistory(e.id)">
            <div class="job-icon">{{ e.status === 'completed' ? 'OK' : 'X' }}</div>
            <div class="job-info">
              <div class="job-name">{{ e.name }}</div>
              <div class="job-meta">
                <span class="tag" [class]="'tag-' + e.status">{{ e.status | uppercase }}</span>
                <span>{{ formatBytes(e.total_bytes) }}</span>
                @if (e.category) { <span>{{ e.category }}</span> }
                <span>{{ formatDate(e.completed_at) }}</span>
                @if (e.error_message) { <span class="error-msg">{{ e.error_message }}</span> }
              </div>
            </div>
            <div class="job-actions">
              @if (e.status === 'failed') {
                <button class="action-btn" (click)="retryHistory(e.id); $event.stopPropagation()">Retry</button>
              }
              <button class="action-btn" (click)="removeHistory(e.id); $event.stopPropagation()">x</button>
            </div>
          </div>

          <!-- Detail panel for history item -->
          @if (selectedId() === e.id) {
            <div class="detail-panel">
              <div class="detail-tabs">
                <button class="dtab" [class.active]="detailTab() === 'info'" (click)="detailTab.set('info')">Info</button>
                <button class="dtab" [class.active]="detailTab() === 'logs'" (click)="detailTab.set('logs'); loadHistoryLogs(e.id)">Logs</button>
              </div>
              @if (detailTab() === 'info') {
                <div class="detail-section">
                  <div class="detail-grid">
                    <div class="dg-label">Status</div><div class="dg-value">{{ e.status | uppercase }}</div>
                    <div class="dg-label">Size</div><div class="dg-value">{{ formatBytes(e.total_bytes) }}</div>
                    <div class="dg-label">Downloaded</div><div class="dg-value">{{ formatBytes(e.downloaded_bytes) }}</div>
                    <div class="dg-label">Added</div><div class="dg-value">{{ formatDate(e.added_at) }}</div>
                    <div class="dg-label">Completed</div><div class="dg-value">{{ formatDate(e.completed_at) }}</div>
                    <div class="dg-label">Duration</div><div class="dg-value">{{ formatDuration(e.added_at, e.completed_at) }}</div>
                    <div class="dg-label">Avg Speed</div><div class="dg-value">{{ avgSpeed(e) }}</div>
                    <div class="dg-label">Category</div><div class="dg-value">{{ e.category || 'None' }}</div>
                    <div class="dg-label">Output</div><div class="dg-value">{{ e.output_dir }}</div>
                    @if (e.error_message) {
                      <div class="dg-label">Error</div><div class="dg-value error-msg">{{ e.error_message }}</div>
                    }
                  </div>
                  @if (e.stages && e.stages.length > 0) {
                    <div class="detail-sub">Processing Stages</div>
                    <div class="stages-list">
                      @for (s of e.stages; track s.name) {
                        <div class="stage-row">
                          <span class="stage-name">{{ s.name }}</span>
                          <span class="tag" [class]="'tag-' + s.status">{{ s.status }}</span>
                          <span class="stage-dur">{{ s.duration_secs.toFixed(1) }}s</span>
                          @if (s.message) { <span class="stage-msg">{{ s.message }}</span> }
                        </div>
                      }
                    </div>
                  }
                  @if (e.server_stats && e.server_stats.length > 0) {
                    <div class="detail-sub">Server Stats</div>
                    <div class="server-stats">
                      @for (ss of e.server_stats; track ss.server_id) {
                        <div class="ss-row">
                          <span class="ss-name">{{ ss.server_name }}</span>
                          <span>{{ ss.articles_downloaded }} articles</span>
                          <span>{{ formatBytes(ss.bytes_downloaded) }}</span>
                          @if (ss.articles_failed > 0) { <span class="error-msg">{{ ss.articles_failed }} failed</span> }
                        </div>
                      }
                    </div>
                  }
                </div>
              }
              @if (detailTab() === 'logs') {
                <div class="log-viewer">
                  @if (jobLogs().loading) { <div class="log-loading">Loading logs...</div>
                  } @else if (jobLogs().entries.length === 0) { <div class="log-empty">No log entries</div>
                  } @else {
                    @for (log of jobLogs().entries; track log.seq) {
                      <div class="log-line" [class]="'log-' + log.level.toLowerCase()">
                        <span class="log-ts">{{ log.timestamp | slice:11:19 }}</span>
                        <span class="log-msg">{{ log.message }}</span>
                      </div>
                    }
                  }
                </div>
              }
            </div>
          }
        }
      </div>
    }
  `,
  styles: [`
    :host { display: flex; flex-direction: column; height: 100%; }
    .queue-toolbar {
      display: flex; align-items: center; justify-content: space-between; padding: 10px 16px;
      background: #0d1117; border-bottom: 1px solid #21262d; font-size: 12px; color: #8b949e;
    }
    .toolbar-btn {
      padding: 4px 12px; border-radius: 4px; border: 1px solid #30363d;
      background: #21262d; color: #c9d1d9; cursor: pointer; font-size: 12px;
    }
    .toolbar-btn:hover { background: #30363d; }
    .toolbar-btn.warn { background: #da3633; border-color: #f85149; color: white; }

    /* Add NZB panel */
    .add-panel { background: #161b22; border-bottom: 1px solid #21262d; padding: 12px 16px; }
    .add-tabs { display: flex; gap: 4px; margin-bottom: 12px; }
    .tab-btn {
      padding: 4px 12px; border-radius: 4px; border: 1px solid #30363d;
      background: #0d1117; color: #8b949e; cursor: pointer; font-size: 12px;
    }
    .tab-btn.active { background: #21262d; color: #c9d1d9; border-color: #58a6ff; }
    .tab-btn:hover { color: #c9d1d9; }
    .add-form { display: flex; flex-direction: column; gap: 8px; }
    .form-row { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
    .form-label { font-size: 12px; color: #8b949e; }
    .form-input {
      flex: 1; min-width: 200px; padding: 4px 8px; border-radius: 4px; border: 1px solid #30363d;
      background: #0d1117; color: #c9d1d9; font-size: 12px; outline: none;
    }
    .form-input:focus { border-color: #58a6ff; }
    .form-select {
      padding: 4px 8px; border-radius: 4px; border: 1px solid #30363d;
      background: #0d1117; color: #c9d1d9; font-size: 12px; outline: none;
    }
    .form-select:focus { border-color: #58a6ff; }
    .file-input { font-size: 12px; color: #c9d1d9; }
    .file-input::file-selector-button {
      padding: 4px 12px; border-radius: 4px; border: 1px solid #30363d;
      background: #21262d; color: #c9d1d9; cursor: pointer; font-size: 12px; margin-right: 8px;
    }
    .file-input::file-selector-button:hover { background: #30363d; }
    .submit-btn {
      padding: 4px 16px; border-radius: 4px; border: 1px solid #238636;
      background: #238636; color: #fff; cursor: pointer; font-size: 12px; font-weight: 600;
    }
    .submit-btn:hover { background: #2ea043; }
    .submit-btn:disabled { opacity: 0.5; cursor: not-allowed; }

    /* Job list */
    .section-list { overflow-y: auto; }
    .section-list:not(.has-history) { flex: 1; }
    .section-list.has-history { max-height: 50%; }
    .history-list { flex: 1; overflow-y: auto; }

    .job-item {
      display: flex; align-items: center; gap: 12px;
      padding: 10px 16px; border-bottom: 1px solid #21262d; cursor: pointer;
    }
    .job-item:hover { background: #161b22; }
    .job-item.selected { background: #161b22; border-left: 3px solid #58a6ff; }
    .history-item .job-icon { font-size: 12px; font-weight: 700; width: 24px; text-align: center; }
    .job-icon { font-size: 14px; width: 24px; text-align: center; color: #8b949e; }
    .job-info { flex: 1; min-width: 0; }
    .job-name { font-weight: 600; font-size: 13px; margin-bottom: 3px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
    .job-meta { display: flex; gap: 10px; font-size: 11px; color: #8b949e; flex-wrap: wrap; align-items: center; }
    .inline-select {
      padding: 1px 4px; border-radius: 4px; border: 1px solid #30363d;
      background: #0d1117; color: #c9d1d9; font-size: 11px; outline: none; cursor: pointer;
    }
    .inline-select:focus { border-color: #58a6ff; }
    .tag { padding: 1px 6px; border-radius: 3px; font-size: 10px; font-weight: 600; }
    .tag-downloading { background: #0d419d; color: #58a6ff; }
    .tag-queued { background: #1c2128; color: #8b949e; }
    .tag-paused { background: #3d1d00; color: #d29922; }
    .tag-verifying, .tag-repairing, .tag-extracting, .tag-post_processing { background: #1a3a1a; color: #3fb950; }
    .tag-completed, .tag-ok { background: #1a3a1a; color: #3fb950; }
    .tag-failed, .tag-error { background: #3d1418; color: #f85149; }
    .tag-skipped { background: #1c2128; color: #8b949e; }
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
      background: #21262d; color: #c9d1d9; cursor: pointer; font-size: 11px;
    }
    .action-btn:hover { background: #30363d; }

    /* History header */
    .history-header {
      display: flex; align-items: center; padding: 8px 16px;
      background: #0d1117; border-top: 1px solid #30363d; border-bottom: 1px solid #21262d;
    }
    .history-title { font-size: 12px; font-weight: 600; color: #8b949e; }
    .spacer { flex: 1; }

    /* Detail panel */
    .detail-panel {
      background: #0d1117; border-bottom: 1px solid #30363d;
      padding: 0 16px 12px 16px;
    }
    .detail-tabs {
      display: flex; gap: 2px; padding: 8px 0; border-bottom: 1px solid #21262d; margin-bottom: 10px;
    }
    .dtab {
      padding: 4px 14px; border-radius: 4px; border: 1px solid transparent;
      background: transparent; color: #8b949e; cursor: pointer; font-size: 12px;
    }
    .dtab:hover { color: #c9d1d9; }
    .dtab.active { background: #21262d; color: #c9d1d9; border-color: #30363d; }

    .detail-section { font-size: 12px; }
    .detail-grid {
      display: grid; grid-template-columns: 100px 1fr; gap: 4px 12px; margin-bottom: 10px;
    }
    .dg-label { color: #8b949e; }
    .dg-value { color: #c9d1d9; word-break: break-all; }
    .error-msg { color: #f85149; }

    .detail-sub { font-size: 11px; font-weight: 600; color: #8b949e; margin: 10px 0 6px; text-transform: uppercase; letter-spacing: 0.5px; }

    .stages-list, .server-stats { font-size: 12px; }
    .stage-row, .ss-row { display: flex; gap: 10px; align-items: center; padding: 3px 0; color: #c9d1d9; }
    .stage-name, .ss-name { color: #c9d1d9; min-width: 80px; }
    .stage-dur { color: #8b949e; }
    .stage-msg { color: #8b949e; font-size: 11px; }

    /* Log viewer */
    .log-viewer {
      font-family: monospace; font-size: 11px; max-height: 300px; overflow-y: auto;
      background: #010409; border: 1px solid #21262d; border-radius: 4px; padding: 8px;
    }
    .log-loading, .log-empty { color: #484f58; padding: 8px 0; }
    .log-line { display: flex; gap: 8px; line-height: 1.5; white-space: pre-wrap; word-break: break-all; }
    .log-ts { color: #484f58; flex-shrink: 0; }
    .log-msg { flex: 1; }
    .log-info .log-msg { color: #8b949e; }
    .log-warn .log-msg { color: #d29922; }
    .log-error .log-msg { color: #f85149; }
    .log-debug .log-msg { color: #6e7681; }

    .empty-state { text-align: center; padding: 64px 16px; color: #484f58; }
    .empty-icon { font-size: 16px; margin-bottom: 8px; font-weight: 600; }
    .hint { font-size: 12px; margin-top: 8px; }
  `],
})
export class QueueViewComponent implements OnInit, OnDestroy {
  jobs = signal<NzbJob[]>([]);
  history = signal<HistoryEntry[]>([]);
  remainingBytes = signal(0);
  categories = signal<CategoryConfig[]>([]);
  selectedId = signal<string | null>(null);
  detailTab = signal<'info' | 'logs'>('info');
  jobLogs = signal<{ entries: LogEntry[]; loading: boolean }>({ entries: [], loading: false });
  private pollTimer: ReturnType<typeof setInterval> | null = null;

  // Add NZB panel state
  showAddPanel = false;
  addMode: 'file' | 'url' = 'file';
  selectedFile: File | null = null;
  addUrl = '';
  addCategory = '';
  addPriority = 1;
  uploading = false;

  constructor(private api: ApiService, private http: HttpClient, private snackBar: MatSnackBar) {}

  ngOnInit(): void {
    this.loadQueue();
    this.loadHistory();
    this.loadCategories();
    this.pollTimer = setInterval(() => { this.loadQueue(); this.loadHistory(); }, 2000);
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

  loadHistory(): void {
    this.api.get<{ entries: HistoryEntry[] }>('/history').subscribe({
      next: r => this.history.set(r.entries || []),
      error: () => {},
    });
  }

  loadCategories(): void {
    this.api.get<CategoryConfig[]>('/config/categories').subscribe({
      next: (cats) => this.categories.set(cats),
      error: () => {},
    });
  }

  // ---- Selection ----

  selectJob(id: string): void {
    if (this.selectedId() === id) { this.selectedId.set(null); return; }
    this.selectedId.set(id);
    this.detailTab.set('info');
    this.jobLogs.set({ entries: [], loading: false });
  }

  selectHistory(id: string): void {
    if (this.selectedId() === id) { this.selectedId.set(null); return; }
    this.selectedId.set(id);
    this.detailTab.set('info');
    this.jobLogs.set({ entries: [], loading: false });
  }

  // ---- Logs ----

  loadJobLogs(id: string): void {
    this.jobLogs.set({ entries: [], loading: true });
    this.api.get<LogsResponse>('/logs', { job_id: id }).subscribe({
      next: r => this.jobLogs.set({ entries: r.entries || [], loading: false }),
      error: () => this.jobLogs.set({ entries: [], loading: false }),
    });
  }

  loadHistoryLogs(id: string): void {
    this.jobLogs.set({ entries: [], loading: true });
    this.api.get<LogsResponse>(`/history/${id}/logs`).subscribe({
      next: r => this.jobLogs.set({ entries: r.entries || [], loading: false }),
      error: () => this.jobLogs.set({ entries: [], loading: false }),
    });
  }

  // ---- Add NZB ----

  onFileSelected(event: Event): void {
    const input = event.target as HTMLInputElement;
    this.selectedFile = input.files?.[0] ?? null;
  }

  uploadFile(): void {
    if (!this.selectedFile || this.uploading) return;
    this.uploading = true;
    const formData = new FormData();
    formData.append('file', this.selectedFile, this.selectedFile.name);
    const params: string[] = [];
    if (this.addCategory) params.push(`category=${encodeURIComponent(this.addCategory)}`);
    if (this.addPriority !== 1) params.push(`priority=${this.addPriority}`);
    const qs = params.length > 0 ? '?' + params.join('&') : '';
    this.http.post(`/api/queue/add${qs}`, formData).subscribe({
      next: () => {
        this.snackBar.open('NZB added to queue', 'Close', { duration: 3000 });
        this.selectedFile = null; this.uploading = false; this.loadQueue();
      },
      error: (err) => {
        this.snackBar.open('Failed: ' + (err.error?.message || err.statusText), 'Close', { duration: 5000 });
        this.uploading = false;
      },
    });
  }

  addFromUrl(): void {
    if (!this.addUrl || this.uploading) return;
    this.uploading = true;
    const body: { url: string; category?: string; priority?: number } = { url: this.addUrl };
    if (this.addCategory) body.category = this.addCategory;
    if (this.addPriority !== 1) body.priority = this.addPriority;
    this.api.post('/queue/add-url', body).subscribe({
      next: () => {
        this.snackBar.open('NZB added from URL', 'Close', { duration: 3000 });
        this.addUrl = ''; this.uploading = false; this.loadQueue();
      },
      error: (err: any) => {
        this.snackBar.open('Failed: ' + (err.error?.message || err.statusText), 'Close', { duration: 5000 });
        this.uploading = false;
      },
    });
  }

  // ---- Per-job actions ----

  changeJobPriority(id: string, priority: number): void {
    this.api.put(`/queue/${id}/priority`, { priority }).subscribe({ next: () => this.loadQueue(), error: () => {} });
  }

  changeJobCategory(id: string, category: string): void {
    this.api.put(`/queue/${id}/category`, { category }).subscribe({ next: () => this.loadQueue(), error: () => {} });
  }

  pauseJob(id: string): void { this.api.post(`/queue/${id}/pause`).subscribe(() => this.loadQueue()); }
  resumeJob(id: string): void { this.api.post(`/queue/${id}/resume`).subscribe(() => this.loadQueue()); }

  deleteJob(id: string): void {
    this.api.delete(`/queue/${id}`).subscribe(() => {
      if (this.selectedId() === id) this.selectedId.set(null);
      this.loadQueue();
    });
  }

  // ---- History actions ----

  retryHistory(id: string): void {
    this.api.post(`/history/${id}/retry`).subscribe(() => {
      this.loadHistory(); this.loadQueue();
      this.snackBar.open('Retrying...', 'Close', { duration: 2000 });
    });
  }

  removeHistory(id: string): void {
    this.api.delete(`/history/${id}`).subscribe(() => {
      if (this.selectedId() === id) this.selectedId.set(null);
      this.loadHistory();
    });
  }

  clearHistory(): void {
    this.api.delete('/history').subscribe(() => {
      this.selectedId.set(null);
      this.loadHistory();
      this.snackBar.open('History cleared', 'Close', { duration: 2000 });
    });
  }

  // ---- Formatting ----

  percent(job: { total_bytes: number; downloaded_bytes: number }): number {
    if (job.total_bytes === 0) return 0;
    return Math.round((job.downloaded_bytes / job.total_bytes) * 100);
  }

  eta(job: NzbJob): string {
    if (job.speed_bps === 0) return '--';
    const remaining = job.total_bytes - job.downloaded_bytes;
    const secs = remaining / job.speed_bps;
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = Math.floor(secs % 60);
    return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}` : `${m}:${String(s).padStart(2, '0')}`;
  }

  avgSpeed(e: HistoryEntry): string {
    const start = new Date(e.added_at).getTime();
    const end = new Date(e.completed_at).getTime();
    const durationSecs = (end - start) / 1000;
    if (durationSecs <= 0) return '--';
    return this.formatSpeed(e.downloaded_bytes / durationSecs);
  }

  formatDuration(start: string, end: string): string {
    const ms = new Date(end).getTime() - new Date(start).getTime();
    if (ms <= 0) return '--';
    const secs = Math.floor(ms / 1000);
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    const s = secs % 60;
    if (h > 0) return `${h}h ${m}m ${s}s`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }

  formatDate(d: string): string {
    if (!d) return '--';
    return new Date(d).toLocaleString();
  }

  statusIcon(status: string): string {
    const icons: Record<string, string> = {
      downloading: 'DL', queued: '..', paused: '||', verifying: 'VR',
      repairing: 'RP', extracting: 'EX', completed: 'OK', failed: 'X',
    };
    return icons[status] || '..';
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
    if (bps === 0) return '0 B/s';
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
