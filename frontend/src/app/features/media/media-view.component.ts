import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';

@Component({
  selector: 'app-media-view',
  standalone: true,
  imports: [CommonModule, MatSnackBarModule],
  template: `
    <div class="panel">
      <h3>Media Library</h3>
      <div class="body">
        <div class="dav-info">
          <div class="info-row">
            <span class="label">WebDAV endpoint</span>
            <code class="url">{{ davUrl }}</code>
            <button class="btn ghost sm" (click)="copyUrl()">Copy</button>
          </div>
          <p class="hint">
            Connect any WebDAV-capable media client to stream content directly from the download
            pipeline — no extraction or waiting required.
          </p>
        </div>
      </div>
    </div>

    <div class="panel">
      <h3>Connecting a client</h3>
      <div class="body">
        <table class="data clients">
          <thead>
            <tr><th>App</th><th>Platform</th><th>How to connect</th></tr>
          </thead>
          <tbody>
            <tr>
              <td><strong>Infuse</strong></td>
              <td>iOS · tvOS · macOS</td>
              <td>Settings → Add Files → Add Network Share → WebDAV → enter URL + credentials</td>
            </tr>
            <tr>
              <td><strong>VLC</strong></td>
              <td>All platforms</td>
              <td>Network → Open Network Stream → paste the WebDAV URL</td>
            </tr>
            <tr>
              <td><strong>Kodi</strong></td>
              <td>All platforms</td>
              <td>Files → Add source → Browse → Add Network Location → WebDAV (HTTP)</td>
            </tr>
            <tr>
              <td><strong>nPlayer</strong></td>
              <td>iOS · tvOS</td>
              <td>+ → WebDAV → enter server URL + credentials</td>
            </tr>
            <tr>
              <td><strong>Windows Explorer</strong></td>
              <td>Windows</td>
              <td>Map Network Drive → paste the WebDAV URL</td>
            </tr>
          </tbody>
        </table>
        <p class="hint cred-note">
          Use the same username and password as the rustnzb web UI.
          Items appear here as they are added via the <strong>▶ media</strong> button in History.
        </p>
      </div>
    </div>
  `,
  styles: [`
    :host { display: block; }
    .dav-info { display: flex; flex-direction: column; gap: 12px; }
    .info-row { display: flex; align-items: center; gap: 10px; flex-wrap: wrap; }
    .info-row .label { color: var(--mute); font-size: 12px; white-space: nowrap; }
    code.url {
      background: var(--panel2); border: 1px solid var(--line);
      border-radius: 4px; padding: 4px 10px; font-size: 13px;
      color: var(--accent); flex: 1; min-width: 0; word-break: break-all;
    }
    .hint { color: var(--mute); font-size: 12px; margin: 0; line-height: 1.6; }
    .cred-note { margin-top: 12px; }
    .btn.sm { padding: 4px 10px; font-size: 12px; flex-shrink: 0; }
    table.clients td { font-size: 13px; }
    table.clients td:last-child { color: var(--mute); font-size: 12px; }
  `],
})
export class MediaViewComponent {
  davUrl = `${window.location.origin}/dav`;

  constructor(private snack: MatSnackBar) {}

  copyUrl(): void {
    navigator.clipboard.writeText(this.davUrl).then(
      () => this.snack.open('URL copied', 'Close', { duration: 2000 }),
      () => this.snack.open(this.davUrl, 'Close', { duration: 5000 }),
    );
  }
}
