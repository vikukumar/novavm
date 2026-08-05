import { existsSync } from 'fs';
import { execSync } from 'child_process';
import { resolve } from 'path';

const rootFrontend = resolve(process.cwd(), 'frontend');
const parentFrontend = resolve(process.cwd(), '../frontend');

const targetDir = existsSync(rootFrontend) ? rootFrontend : existsSync(parentFrontend) ? parentFrontend : 'frontend';
console.log(`[NovaVM Build] Building frontend in: ${targetDir}`);
execSync(`npm --prefix "${targetDir}" run build`, { stdio: 'inherit' });
