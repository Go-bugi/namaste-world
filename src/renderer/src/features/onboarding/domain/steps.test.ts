import { fc, test } from '@fast-check/vitest';
import { describe, expect, it } from 'vitest';
import { stepStatuses } from './steps';

describe('stepStatuses', () => {
  it('FR-START-002 시나리오 1: 앱 시작 감지 중에는 감지만 진행 중이고 나머지는 시작 전이다', () => {
    expect(
      stepStatuses({
        detecting: true,
        hasDetection: false,
        installing: false,
        hasInstallation: false,
      }),
    ).toEqual({ detect: 'running', install: 'pending', select: 'pending' });
  });

  it('FR-START-002 시나리오 1: 감지가 끝나고 훅 설치 중이면 설치가 진행 중이다', () => {
    expect(
      stepStatuses({
        detecting: false,
        hasDetection: true,
        installing: true,
        hasInstallation: false,
      }),
    ).toEqual({ detect: 'done', install: 'running', select: 'pending' });
  });

  it('FR-START-002 시나리오 2: 감지와 훅 설치가 끝나면 둘은 끝났고 선택 단계가 진행 중이다', () => {
    expect(
      stepStatuses({
        detecting: false,
        hasDetection: true,
        installing: false,
        hasInstallation: true,
      }),
    ).toEqual({ detect: 'done', install: 'done', select: 'running' });
  });

  it('FR-HOOK-015 시나리오 2: 다시 감지 뒤 다시 설치 중이면 이전 결과가 있어도 설치 단계는 진행 중이다', () => {
    const got = stepStatuses({
      detecting: false,
      hasDetection: true,
      installing: true,
      hasInstallation: true,
    });

    expect(got).toEqual({ detect: 'done', install: 'running', select: 'pending' });
  });

  it('FR-AGENT-008 시나리오 3: 다시 감지 중이면 이전 결과가 있어도 감지는 진행 중이고 훅 설치와 선택은 대기로 돌아간다', () => {
    expect(
      stepStatuses({
        detecting: true,
        hasDetection: true,
        installing: false,
        hasInstallation: true,
      }),
    ).toEqual({ detect: 'running', install: 'pending', select: 'pending' });
  });

  test.prop([fc.boolean(), fc.boolean(), fc.boolean()])(
    'FR-START-002: 감지 중이면 다른 입력과 무관하게 감지만 진행 중이고 나머지는 대기다',
    (hasDetection, installing, hasInstallation) => {
      expect(stepStatuses({ detecting: true, hasDetection, installing, hasInstallation })).toEqual({
        detect: 'running',
        install: 'pending',
        select: 'pending',
      });
    },
  );
});
