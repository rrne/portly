// cwd + 풀 명령줄에서 "사람이 이해하는 이름"을 만든다.
// 예: cwd=/Users/coco/Desktop/factory/portal, cmd="node .../vite/bin/vite.js"
//     → label="portal", framework="vite"
import type { ProcessMeta } from "./types";

// 풀 명령줄에서 알아볼 프레임워크/러너 (부분일치, 위에서부터 우선)
const FRAMEWORK_HINTS: Array<[RegExp, string]> = [
  [/next(\/|-|\s)/i, "Next.js"],
  [/vite/i, "Vite"],
  [/nuxt/i, "Nuxt"],
  [/webpack/i, "webpack"],
  [/remix/i, "Remix"],
  [/astro/i, "Astro"],
  [/nest/i, "NestJS"],
  [/tauri/i, "Tauri"],
  [/postgres/i, "PostgreSQL"],
  [/mysqld|mysql/i, "MySQL"],
  [/redis/i, "Redis"],
  [/mongod/i, "MongoDB"],
  [/gradle|\.jar\b|spring/i, "Java/Spring"],
  [/uvicorn|gunicorn|flask|django|manage\.py/i, "Python"],
  [/rails|puma/i, "Rails"],
];

export interface Friendly {
  label: string; // 폴더명 등 사람이 읽는 이름
  framework?: string; // 추론한 프레임워크
  cwd?: string;
}

/** cwd 경로에서 마지막 폴더명을 뽑는다. (/a/b/portal → "portal") */
function folderName(cwd: string): string {
  const parts = cwd.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || cwd;
}

function detectFramework(fullCommand: string): string | undefined {
  for (const [re, name] of FRAMEWORK_HINTS) {
    if (re.test(fullCommand)) return name;
  }
  return undefined;
}

/** ProcessMeta → 사람이 읽는 이름. cwd가 없으면 명령줄에서 최대한 뽑는다. */
export function toFriendly(meta: ProcessMeta): Friendly {
  const framework = detectFramework(meta.fullCommand);
  let label = "";

  if (meta.cwd && meta.cwd !== "/") {
    label = folderName(meta.cwd);
  } else {
    // cwd를 못 얻으면 명령줄에서 실행 파일명만 뽑는다.
    // (--seatbelt-client=43 같은 긴 플래그 노이즈는 버린다)
    const firstToken = meta.fullCommand.trim().split(/\s+/)[0] ?? "";
    label = folderName(firstToken) || "unknown";
  }

  return { label, framework, cwd: meta.cwd || undefined };
}
