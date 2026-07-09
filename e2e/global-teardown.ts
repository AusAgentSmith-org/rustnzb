import * as path from 'path';
import * as fs from 'fs';
import { stopAllBackends, cleanAllBackendData } from './helpers/backend';
import { stopMockNntp } from './helpers/mock-nntp';

export default async function globalTeardown() {
  stopAllBackends();
  stopMockNntp();
  cleanAllBackendData();
  const stateFile = path.join(__dirname, 'auth-state.json');
  if (fs.existsSync(stateFile)) fs.unlinkSync(stateFile);
  const mockStateFile = path.join(__dirname, 'mock-auth-state.json');
  if (fs.existsSync(mockStateFile)) fs.unlinkSync(mockStateFile);
}
