import { Routes } from '@angular/router';

export const routes: Routes = [
  { path: '', redirectTo: '/queue', pathMatch: 'full' },
  { path: 'queue', loadComponent: () => import('./features/queue/queue-view.component').then(m => m.QueueViewComponent) },
  { path: 'groups', loadComponent: () => import('./features/groups/groups-view.component').then(m => m.GroupsViewComponent) },
  { path: 'history', loadComponent: () => import('./features/history/history-view.component').then(m => m.HistoryViewComponent) },
  { path: 'rss', loadComponent: () => import('./features/rss/rss-view.component').then(m => m.RssViewComponent) },
  { path: 'settings', loadComponent: () => import('./features/settings/settings-view.component').then(m => m.SettingsViewComponent) },
  { path: 'logs', loadComponent: () => import('./features/logs/logs-view.component').then(m => m.LogsViewComponent) },
];
