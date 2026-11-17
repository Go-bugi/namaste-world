import { fc, test } from '@fast-check/vitest';
import type { ShellKind } from '@renderer/types/bindings/ShellKind';
import { describe, expect, it } from 'vitest';
import { shellDescription, shellLabel, shouldDetect } from './shell';

const ALL: readonly ShellKind[] = ['powerShell', 'cmd', 'gitBash'];

const shellArb = fc.constantFrom<ShellKind>(...ALL);

describe('shellLabel', () => {
  it('FR-START-011: 선택지 이름은 PowerShell, 명령 프롬프트, Git Bash다', () => {
    expect(shellLabel('powerShell')).toBe('PowerShell');
    expect(shellLabel('cmd')).toBe('명령 프롬프트');
    expect(shellLabel('gitBash')).toBe('Git Bash');
  });
});

describe('shellDescription', () => {
  it('FR-START-011: 선택지마다 그 셸의 설정 파일이 전달되는지 한 줄로 알려 준다', () => {
    expect(shellDescription('powerShell')).toBe('터미널의 PowerShell과 같은 설정으로 실행합니다');
    expect(shellDescription('cmd')).toBe('Windows 기본 설정으로만 실행합니다');
    expect(shellDescription('gitBash')).toBe('터미널의 Git Bash와 같은 설정으로 실행합니다');
  });
});

describe('shouldDetect', () => {
  it('FR-START-013 시나리오 1: 아직 감지하지 않았으면 다음을 누를 때 감지한다', () => {
    for (const shell of ALL) expect(shouldDetect(undefined, shell)).toBe(true);
  });

  it('FR-START-013 시나리오 3: PowerShell로 감지한 뒤 Git Bash를 고르고 다음을 누르면 다시 감지한다', () => {
    expect(shouldDetect('powerShell', 'gitBash')).toBe(true);
  });

  it('FR-START-013 시나리오 5: PowerShell로 감지한 뒤 PowerShell 그대로 다음을 누르면 다시 감지하지 않는다', () => {
    expect(shouldDetect('powerShell', 'powerShell')).toBe(false);
  });

  test.prop([fc.option(shellArb, { nil: undefined }), shellArb])(
    'FR-START-013: 마지막 감지와 셸이 다르거나 감지한 적이 없을 때만 감지한다',
    (lastDetected, selected) => {
      expect(shouldDetect(lastDetected, selected)).toBe(lastDetected !== selected);
    },
  );
});
