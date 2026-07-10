import '@angular/compiler';

import { HttpClient } from '@angular/common/http';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { of } from 'rxjs';

import { AuthService, TokenResponse } from './auth.service';

const TOKENS: TokenResponse = {
  access_token: 'access-1',
  refresh_token: 'refresh-1',
  token_type: 'Bearer',
  expires_in: 3600,
};

describe('AuthService', () => {
  let http: { get: ReturnType<typeof vi.fn>; post: ReturnType<typeof vi.fn> };
  let service: AuthService;

  beforeEach(() => {
    localStorage.clear();
    http = {
      get: vi.fn(() => of({ auth_enabled: true, setup_required: false })),
      post: vi.fn(() => of(TOKENS)),
    };
    service = new AuthService(http as unknown as HttpClient);
  });

  it('queries the public authentication status endpoint', () => {
    service.checkAuth().subscribe();
    expect(http.get).toHaveBeenCalledWith('/api/auth/status');
  });

  it.each([
    ['setup', 'setup'],
    ['login', 'login'],
  ] as const)('%s posts credentials and stores returned tokens', (method, endpoint) => {
    service[method]('alice', 'secret').subscribe();

    expect(http.post).toHaveBeenCalledWith(`/api/auth/${endpoint}`, {
      username: 'alice',
      password: 'secret',
    });
    expect(service.getAccessToken()).toBe('access-1');
    expect(localStorage.getItem('refresh_token')).toBe('refresh-1');
  });

  it('refreshes with the persisted refresh token and rotates both tokens', () => {
    localStorage.setItem('refresh_token', 'old-refresh');
    service.refresh().subscribe();

    expect(http.post).toHaveBeenCalledWith('/api/auth/refresh', {
      refresh_token: 'old-refresh',
    });
    expect(localStorage.getItem('refresh_token')).toBe('refresh-1');
  });

  it('clears local credentials immediately when logging out', () => {
    localStorage.setItem('access_token', 'old-access');
    localStorage.setItem('refresh_token', 'old-refresh');
    service.logout().subscribe();

    expect(http.post).toHaveBeenCalledWith('/api/auth/logout', {
      refresh_token: 'old-refresh',
    });
    expect(service.isLoggedIn()).toBe(false);
    expect(localStorage.getItem('refresh_token')).toBeNull();
  });

  it('reports login state solely from the access token', () => {
    expect(service.isLoggedIn()).toBe(false);
    localStorage.setItem('refresh_token', 'refresh-only');
    expect(service.isLoggedIn()).toBe(false);
    localStorage.setItem('access_token', 'access');
    expect(service.isLoggedIn()).toBe(true);
  });
});
