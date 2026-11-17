import type { ShellKind } from '@renderer/types/bindings/ShellKind';

/** 셸 선택지 이름. 'PowerShell' | '명령 프롬프트' | 'Git Bash' */
export function shellLabel(shell: ShellKind): string {
  switch (shell) {
    case 'powerShell':
      return 'PowerShell';
    case 'cmd':
      return '명령 프롬프트';
    case 'gitBash':
      return 'Git Bash';
  }
}

/** 셸 설정 파일의 환경 변수가 전달되는지 한 줄로 알려 주는 설명. FR-START-011 */
export function shellDescription(shell: ShellKind): string {
  switch (shell) {
    case 'powerShell':
      return '터미널의 PowerShell과 같은 설정으로 실행합니다';
    case 'cmd':
      return 'Windows 기본 설정으로만 실행합니다';
    case 'gitBash':
      return '터미널의 Git Bash와 같은 설정으로 실행합니다';
  }
}

/** 다음을 눌렀을 때 감지를 부를지. 아직 감지하지 않았거나 마지막 감지와 셸이 다르면 true. FR-START-013 */
export function shouldDetect(lastDetected: ShellKind | undefined, selected: ShellKind): boolean {
  return lastDetected !== selected;
}
