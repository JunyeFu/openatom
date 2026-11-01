// Controlled component props/transport only; this is not an actual Host or generation result.
import { createRoot } from 'react-dom/client';
import { FactoryCheckWorkspace } from '../../../apps/planner/src/features/factory/FactoryCheckWorkspace';
import '../../../apps/planner/src/styles/workbench.css';
const fixture = await fetch('./fixtures/b60/balanced.input.json').then(response => response.json());
const project = fixture.project, goal = fixture.scenario.acceptance_goal;
if (new URLSearchParams(location.search).has('single')) delete goal.additional_acceptance_scopes;
project.generation_context = { goal, template_ref:null, parameter_sources:[], draft_check:null, initial_run_id:null, revised_run_ids:[], changes:[], stop_reason:null };
const requests:unknown[] = [];
(window as any).__p201GoalRequests = requests;
(window as any).__TAURI_INTERNALS__ = { async invoke(command:string,args:any) {
  if(command !== 'project_api') throw new Error('Unexpected controlled-test transport operation');
  requests.push(args.request);return {outcome:{status:'ok',data:{project_revision:project.revision,scope:args.request.params.check_scope.objects,checks:[],issues:[]}}};
}};
createRoot(document.getElementById('root')!).render(<FactoryCheckWorkspace project={project} supported selection={[]} busy={false} onClose={()=>{}} onLocate={()=>{}} onDirtyChange={()=>{}} />);
