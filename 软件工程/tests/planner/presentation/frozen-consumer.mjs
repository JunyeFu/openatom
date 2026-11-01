// Compile the frozen production projection/scene modules unchanged. Native
// defaults are unavailable in this non-GUI process; only injected captured data
// ports are exercised by projectFactoryPresentation, which has no dispatcher.
import {readFile,writeFile} from 'node:fs/promises';
import {stripTypeScriptTypes} from 'node:module';
import {root,out} from './runtime.mjs';
const files={
 'load':'apps/planner/src/features/media/load.ts',
 'imageSurface':'apps/planner/src/features/media/imageSurface.ts',
 'visualCapture':'apps/planner/src/features/render-surface/visualCapture.ts',
 'factoryMedia':'apps/planner/src/scene/factoryMedia.ts',
 'factoryItems':'apps/planner/src/scene/factoryItems.ts',
 'factoryResources':'apps/planner/src/scene/factoryResources.ts',
 'factoryPresentation':'apps/planner/src/services/factoryPresentation.ts',
 'presentationHost':'apps/planner/src/services/presentationHost.ts',
 'frozenProcess':'apps/planner/src/services/frozenProcess.ts',
 'layout':'apps/planner/src/scene/layout.ts',
};
for(const [name,path]of Object.entries(files)){
 let text=stripTypeScriptTypes(await readFile(new URL(path,root),'utf8'),{mode:'transform'});
 text=text.replace(/import\s*\{\s*dispatch\s*\}\s*from\s*['"]\.\/projectHost['"];?/g,"const dispatch=async()=>{throw Error('Native default is not used by the captured-input projection');};")
 .replace(/import\s*\{\s*readMediaAsset\s*\}\s*from\s*['"]\.\/mediaAssets['"];?/g,"const readMediaAsset=async()=>{throw Error('Use the exact captured Host asset port');};");
 text=text.replace(/from\s+['"]([^'"]+)['"]/g,(whole,ref)=>{
  if(ref.endsWith('/packages/presentation/src'))return "from './index.mjs'";
  const target=ref.split('/').at(-1);return files[target]?`from './${target}.mjs'`:whole;
 });
 await writeFile(new URL(name+'.mjs',out),text);
}
export {root,out};
