import { Injectable, inject } from '@angular/core';
import { MatDialog } from '@angular/material/dialog';
import { Observable, map } from 'rxjs';
import { ConfirmDialogComponent, ConfirmDialogData } from './confirm-dialog.component';

/**
 * In-theme replacement for the browser's native confirm() — used for every
 * destructive action (delete server/category/feed/rule/job, clear history)
 * so the confirmation doesn't look like an OS popup dropped into a dark UI.
 */
@Injectable({ providedIn: 'root' })
export class ConfirmService {
  private dialog = inject(MatDialog);

  confirm(data: ConfirmDialogData): Observable<boolean> {
    return this.dialog
      .open(ConfirmDialogComponent, { data, width: '380px', autoFocus: 'dialog' })
      .afterClosed()
      .pipe(map((result) => result === true));
  }
}
