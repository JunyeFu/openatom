import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseArgs } from 'node:util';
import { spawn } from 'node:child_process';

const root = fileURLToPath(new URL('../../', import.meta.url));
const { values } = parseArgs({ options: {
  kind: { type: 'string' },
  file: { type: 'string', multiple: true },
  test: { type: 'string', multiple: true },
  package: { type: 'string' },
  filter: { type: 'string' },
  run: { type: 'boolean' },
  help: { type: 'boolean' },
} });

async function main() {
  if (values.help) {
    console.log('verify --kind docs|node|frontend|rust [--file PATH ...] [--test PATH ...] [--package CRATE] [--filter NAME] [--run]');
    console.log('默认仅列计划。docs只读明确文件并解析JSON；frontend执行现有typecheck；rust必须指定包。无Native/正式验收结论。');
    return 0;
  }
  const tests = values.test ?? [];
  const files = values.file ?? [];
  const commands = [];
  switch (values.kind) {
    case 'docs':
      if (!files.length) throw new Error('docs需要至少一个--file，不扫描整个仓库。');
      break;
    case 'node':
      if (!tests.length) throw new Error('node需要明确--test，不默认全套测试。');
      commands.push({ file: process.execPath, args: ['--test', ...tests] });
      break;
    case 'frontend':
      commands.push({ file: process.execPath, args: ['--run', 'typecheck'] });
      if (tests.length) commands.push({ file: process.execPath, args: ['--test', ...tests] });
      break;
    case 'rust':
      if (!values.package) throw new Error('rust需要--package；全workspace回归使用既有显式入口。');
      commands.push({ file: 'cargo', args: ['test', '--locked', '-p', values.package, ...(values.filter ? [values.filter] : [])] });
      break;
    default:
      throw new Error('请指定--kind docs|node|frontend|rust。');
  }
  // 不静默接受与所选模式无关的参数，以免误以为该行为已经检查。
  if (values.kind !== 'docs' && files.length) throw new Error('--file仅用于docs。');
  if (!['node', 'frontend'].includes(values.kind) && tests.length) throw new Error('--test仅用于node/frontend。');
  if (values.kind !== 'rust' && (values.package || values.filter)) throw new Error('--package/--filter仅用于rust。');
  console.log(JSON.stringify({
    mode: values.run ? 'execute' : 'plan', kind: values.kind, cwd: root,
    files, commands, scope: '仅所列检查；不提供Native、正式矩阵或整包验收结论。',
  }, null, 2));
  if (!values.run) return 0;
  if (values.kind === 'docs') {
    for (const file of files) {
      const contents = await readFile(path.resolve(root, file), 'utf8');
      if (path.extname(file).toLowerCase() === '.json') JSON.parse(contents);
    }
    console.log(`已读取${files.length}个明确文件并解析其中JSON；未判定文档语义或链接。`);
    return 0;
  }
  for (const command of commands) {
    // Node的测试工作进程标记会让嵌套--test跳过执行并返回0。
    // 此处明确启动新的测试进程，不能继承父测试器的内部身份。
    const env = { ...process.env };
    if (command.file === process.execPath && command.args[0] === '--test') delete env.NODE_TEST_CONTEXT;
    const code = await new Promise((resolve, reject) => {
      const child = spawn(command.file, command.args, { cwd: root, stdio: 'inherit', shell: false, env });
      child.once('error', reject);
      child.once('exit', (exitCode, signal) => signal ? reject(new Error(`检查被${signal}中止`)) : resolve(exitCode));
    });
    if (code !== 0) return code;
  }
  return 0;
}

try {
  process.exitCode = await main();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
