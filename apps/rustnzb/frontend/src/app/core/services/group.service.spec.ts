import '@angular/compiler';

import { describe, expect, it, vi } from 'vitest';
import { of } from 'rxjs';

import { ApiService } from './api.service';
import { GroupService } from './group.service';

describe('GroupService', () => {
  function setup() {
    const api = { get: vi.fn(() => of({})), post: vi.fn(() => of({})) };
    return {
      api,
      service: new GroupService(api as unknown as ApiService),
    };
  }

  it('serializes group list filters, including false subscription state', () => {
    const { api, service } = setup();
    service.list({ subscribed: false, search: 'alt.test', limit: 50, offset: 25 }).subscribe();
    expect(api.get).toHaveBeenCalledWith('/groups', {
      subscribed: 'false',
      search: 'alt.test',
      limit: '50',
      offset: '25',
    });
  });

  it('uses stable routes for group lifecycle operations', () => {
    const { api, service } = setup();
    service.refresh().subscribe();
    service.subscribe(7).subscribe();
    service.unsubscribe(7).subscribe();
    service.markAllRead(7).subscribe();

    expect(api.post.mock.calls.map((call) => (call as unknown[])[0])).toEqual([
      '/groups/refresh',
      '/groups/7/subscribe',
      '/groups/7/unsubscribe',
      '/groups/7/headers/mark-all-read',
    ]);
  });

  it('serializes header filters and download payloads', () => {
    const { api, service } = setup();
    service.listHeaders(9, { search: 'release', limit: 100, offset: 200 }).subscribe();
    service.downloadSelected(9, ['<one@test>', 'two@test'], 'Release', 'tv').subscribe();

    expect(api.get).toHaveBeenCalledWith('/groups/9/headers', {
      search: 'release',
      limit: '100',
      offset: '200',
    });
    expect(api.post).toHaveBeenCalledWith('/groups/9/headers/download', {
      message_ids: ['<one@test>', 'two@test'],
      name: 'Release',
      category: 'tv',
    });
  });

  it('URL-encodes message IDs when requesting an article', () => {
    const { api, service } = setup();
    service.getArticle('<id+tag@example.test>').subscribe();
    expect(api.get).toHaveBeenCalledWith('/articles/%3Cid%2Btag%40example.test%3E');
  });
});
