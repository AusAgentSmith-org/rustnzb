import { Component, Inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';

export interface ConfirmDialogData {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  /** Styles the confirm button as a destructive (red) action instead of primary (blue). */
  danger?: boolean;
}

@Component({
  selector: 'app-confirm-dialog',
  standalone: true,
  imports: [CommonModule, MatDialogModule],
  template: `
    <h2 mat-dialog-title>{{ data.title }}</h2>
    <mat-dialog-content>
      <p class="message">{{ data.message }}</p>
    </mat-dialog-content>
    <mat-dialog-actions align="end">
      <button class="btn" (click)="dialogRef.close(false)">{{ data.cancelLabel || 'Cancel' }}</button>
      <button
        class="btn"
        [class.danger]="data.danger"
        [class.primary]="!data.danger"
        (click)="dialogRef.close(true)"
      >
        {{ data.confirmLabel || 'Confirm' }}
      </button>
    </mat-dialog-actions>
  `,
  styles: [
    `
      h2[mat-dialog-title] {
        color: var(--text);
        margin: 0 0 4px;
        font-size: 16px;
      }
      .message {
        color: var(--mute);
        font-size: 13px;
        margin: 0;
        line-height: 1.5;
        max-width: 340px;
      }
      mat-dialog-actions {
        padding-top: 12px;
      }
    `,
  ],
})
export class ConfirmDialogComponent {
  constructor(
    public dialogRef: MatDialogRef<ConfirmDialogComponent, boolean>,
    @Inject(MAT_DIALOG_DATA) public data: ConfirmDialogData,
  ) {}
}
