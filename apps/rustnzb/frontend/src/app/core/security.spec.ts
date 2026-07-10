import '@angular/compiler';

import {
  HttpErrorResponse,
  HttpRequest,
  HttpResponse,
  HttpHandlerFn,
} from '@angular/common/http';
import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { firstValueFrom, of, throwError } from 'rxjs';

import { authGuard } from './guards/auth.guard';
import { authInterceptor } from './interceptors/auth.interceptor';
import { AuthService } from './services/auth.service';

describe('authGuard', () => {
  function run(loggedIn: boolean) {
    const auth = { isLoggedIn: vi.fn(() => loggedIn) };
    const router = { navigate: vi.fn(() => Promise.resolve(true)) };
    TestBed.configureTestingModule({
      providers: [
        { provide: AuthService, useValue: auth },
        { provide: Router, useValue: router },
      ],
    });
    const result = TestBed.runInInjectionContext(() => authGuard({} as never, {} as never));
    return { result, router };
  }

  afterEach(() => TestBed.resetTestingModule());

  it('allows authenticated navigation', () => {
    const { result, router } = run(true);
    expect(result).toBe(true);
    expect(router.navigate).not.toHaveBeenCalled();
  });

  it('redirects anonymous navigation to login', () => {
    const { result, router } = run(false);
    expect(result).toBe(false);
    expect(router.navigate).toHaveBeenCalledWith(['/login']);
  });
});

describe('authInterceptor', () => {
  function configure(refreshResult = of({ access_token: 'new-access' })) {
    const auth = {
      refresh: vi.fn(() => refreshResult),
      clearTokens: vi.fn(),
    };
    const router = { navigate: vi.fn(() => Promise.resolve(true)) };
    TestBed.configureTestingModule({
      providers: [
        { provide: AuthService, useValue: auth },
        { provide: Router, useValue: router },
      ],
    });
    return { auth, router };
  }

  afterEach(() => TestBed.resetTestingModule());

  it('does not intercept authentication endpoints', async () => {
    const next = vi.fn(() => of(new HttpResponse({ status: 200 }))) as unknown as HttpHandlerFn;
    const request = new HttpRequest('POST', '/api/auth/login', {});
    await firstValueFrom(authInterceptor(request, next));
    expect(next).toHaveBeenCalledWith(request);
  });

  it('refreshes after a 401 and retries with the rotated access token', async () => {
    const { auth } = configure();
    const next = vi
      .fn()
      .mockReturnValueOnce(throwError(() => new HttpErrorResponse({ status: 401 })))
      .mockReturnValueOnce(of(new HttpResponse({ status: 200 })));
    const request = new HttpRequest('GET', '/api/queue');

    await firstValueFrom(
      TestBed.runInInjectionContext(() => authInterceptor(request, next as HttpHandlerFn)),
    );

    expect(auth.refresh).toHaveBeenCalledTimes(1);
    expect(next).toHaveBeenCalledTimes(2);
    expect((next.mock.calls[1][0] as HttpRequest<unknown>).headers.get('Authorization')).toBe(
      'Bearer new-access',
    );
  });

  it('clears credentials and redirects when refresh fails', async () => {
    const { auth, router } = configure(
      throwError(() => new HttpErrorResponse({ status: 403 })),
    );
    const next = vi.fn(() => throwError(() => new HttpErrorResponse({ status: 401 })));

    await expect(
      firstValueFrom(
        TestBed.runInInjectionContext(() =>
          authInterceptor(new HttpRequest('GET', '/api/queue'), next as HttpHandlerFn),
        ),
      ),
    ).rejects.toMatchObject({ status: 403 });
    expect(auth.clearTokens).toHaveBeenCalledTimes(1);
    expect(router.navigate).toHaveBeenCalledWith(['/login']);
  });
});
