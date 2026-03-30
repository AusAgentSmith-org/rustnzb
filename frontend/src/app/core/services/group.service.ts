import { Injectable } from '@angular/core';
import { Observable } from 'rxjs';
import { ApiService } from './api.service';
import { GroupListResponse, GroupStatusResponse, HeaderListResponse } from '../models/group.model';

@Injectable({ providedIn: 'root' })
export class GroupService {
  constructor(private api: ApiService) {}

  list(params: { subscribed?: boolean; search?: string; limit?: number; offset?: number } = {}): Observable<GroupListResponse> {
    const q: Record<string, string> = {};
    if (params.subscribed !== undefined) q['subscribed'] = String(params.subscribed);
    if (params.search) q['search'] = params.search;
    if (params.limit) q['limit'] = String(params.limit);
    if (params.offset) q['offset'] = String(params.offset);
    return this.api.get<GroupListResponse>('/groups', q);
  }

  refresh(): Observable<{ status: boolean; message: string; total: number }> {
    return this.api.post('/groups/refresh');
  }

  getStatus(id: number): Observable<GroupStatusResponse> {
    return this.api.get(`/groups/${id}/status`);
  }

  subscribe(id: number): Observable<unknown> {
    return this.api.post(`/groups/${id}/subscribe`);
  }

  unsubscribe(id: number): Observable<unknown> {
    return this.api.post(`/groups/${id}/unsubscribe`);
  }

  listHeaders(groupId: number, params: { search?: string; limit?: number; offset?: number } = {}): Observable<HeaderListResponse> {
    const q: Record<string, string> = {};
    if (params.search) q['search'] = params.search;
    if (params.limit) q['limit'] = String(params.limit);
    if (params.offset) q['offset'] = String(params.offset);
    return this.api.get<HeaderListResponse>(`/groups/${groupId}/headers`, q);
  }

  fetchHeaders(groupId: number): Observable<{ status: boolean; message: string }> {
    return this.api.post(`/groups/${groupId}/headers/fetch`);
  }

  markAllRead(groupId: number): Observable<{ marked: number }> {
    return this.api.post(`/groups/${groupId}/headers/mark-all-read`);
  }

  downloadSelected(groupId: number, messageIds: string[], name?: string, category?: string): Observable<{ status: boolean; job_id: string; message: string }> {
    return this.api.post(`/groups/${groupId}/headers/download`, { message_ids: messageIds, name, category });
  }

  getArticle(messageId: string): Observable<{ message_id: string; code: number; message: string; body: string | null }> {
    return this.api.get(`/articles/${encodeURIComponent(messageId)}`);
  }
}
