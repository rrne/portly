// 프로세스를 "내 dev 서버 / 시스템 / 기타"로 분류한다.
// 판단은 전부 여기(TS)에서. 백엔드는 lsof 결과만 넘긴다(얇은 백엔드 원칙).
import type { PortProcess, DecoratedProcess, ProcessGroup } from "./types";

// 내 dev 서버로 볼 명령어 시그니처 (소문자 부분일치)
const DEV_SIGNATURES = [
  "node",
  "next",
  "vite",
  "esbuild",
  "pnpm",
  "npm",
  "yarn",
  "bun",
  "deno",
  "webpack",
  "rollup",
  "turbo",
  "python",
  "python3",
  "ruby",
  "rails",
  "java",
  "gradle",
  "php",
  "cargo",
  "go",
];

// macOS 시스템/백그라운드 데몬으로 볼 명령어 (부분일치)
const SYSTEM_SIGNATURES = [
  "rapportd",
  "identityservices",
  "identitys",
  "controlce", // ControlCenter
  "sharingd",
  "remoted",
  "launchd",
  "mdns",
  "airplay",
  "netbiosd",
  "cupsd",
  "coreaudio",
];

// 끄면 데이터 손실/서비스 중단 위험이 있는 인프라 (부분일치)
const RISKY_SIGNATURES = [
  "postgres",
  "mysqld",
  "mysql",
  "redis",
  "mongod",
  "docker",
  "com.docker",
  "elasticsearch",
  "rabbitmq",
  "kafka",
];

export type Risk = "none" | "data" | "system";

/** kill 위험도를 판정한다. system=시스템 불안정, data=데이터 손실 위험. */
export function riskOf(p: PortProcess, group: ProcessGroup): Risk {
  if (group === "system") return "system";
  const cmd = p.command.toLowerCase();
  if (RISKY_SIGNATURES.some((s) => cmd.includes(s))) return "data";
  return "none";
}

/** cwd가 내 루트(roots) 하위인지. roots가 비면 홈(~) 하위로 본다. */
export function isUnderMyRoots(
  cwd: string | undefined,
  roots: string[],
  home: string,
): boolean {
  if (!cwd || cwd === "/") return false;
  const bases = roots.length > 0 ? roots : [home];
  return bases.some((base) => {
    const b = base.replace(/\/+$/, "");
    return cwd === b || cwd.startsWith(b + "/");
  });
}

/**
 * 프로세스 하나를 그룹으로 분류.
 * cwd가 "내 루트 하위"이면(=내가 띄운 것) dev로 강하게 판정한다.
 */
export function classify(
  p: PortProcess,
  me: string,
  opts: { cwd?: string; roots: string[]; home: string },
): ProcessGroup {
  const cmd = p.command.toLowerCase();

  // root 소유거나 알려진 시스템 데몬 → 시스템 (건드리면 안 되는 것)
  if (p.user === "root" || SYSTEM_SIGNATURES.some((s) => cmd.includes(s))) {
    return "system";
  }
  // cwd가 내 루트 하위 → 내가 띄운 dev (가장 강한 신호)
  if (isUnderMyRoots(opts.cwd, opts.roots, opts.home)) {
    return "dev";
  }
  // cwd를 아직 모르지만(메타 미보강) dev 시그니처가 있으면 잠정 dev
  if (!opts.cwd && p.user === me && DEV_SIGNATURES.some((s) => cmd.includes(s))) {
    return "dev";
  }
  // 그 외
  return "other";
}

export interface GroupedProcesses {
  dev: DecoratedProcess[];
  other: DecoratedProcess[];
  system: DecoratedProcess[];
}

/**
 * 목록 전체를 3그룹으로 나눈다. 각 그룹 안은 포트 오름차순.
 * cwdByPid: pid→cwd (process_meta 결과). roots/home: 필터 기준.
 */
export function groupAll(
  procs: PortProcess[],
  me: string,
  cwdByPid: Record<number, string | undefined>,
  roots: string[],
  home: string,
): GroupedProcesses {
  const decorated: DecoratedProcess[] = procs.map((p) => {
    const cwd = cwdByPid[p.pid];
    return {
      ...p,
      cwd,
      group: classify(p, me, { cwd, roots, home }),
      isMine: false, // P5에서 Portal이 띄운 것 매칭
    };
  });

  const byPort = (a: DecoratedProcess, b: DecoratedProcess) => a.port - b.port;

  return {
    dev: decorated.filter((d) => d.group === "dev").sort(byPort),
    other: decorated.filter((d) => d.group === "other").sort(byPort),
    system: decorated.filter((d) => d.group === "system").sort(byPort),
  };
}

/**
 * "나"(현재 사용자)를 추정한다. lsof는 소유자명을 주지만 앱은 현재 사용자를 모르니,
 * root가 아닌 프로세스 중 가장 많은 소유자를 "나"로 본다(대개 로그인 계정).
 */
export function guessMe(procs: PortProcess[]): string {
  const counts = new Map<string, number>();
  for (const p of procs) {
    if (p.user === "root") continue;
    counts.set(p.user, (counts.get(p.user) ?? 0) + 1);
  }
  let best = "";
  let max = -1;
  for (const [user, n] of counts) {
    if (n > max) {
      max = n;
      best = user;
    }
  }
  return best;
}
