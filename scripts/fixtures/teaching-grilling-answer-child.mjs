import {readFile} from 'node:fs/promises';
import {join} from 'node:path';
import {createPlaybookClient} from '../../packages/client/src/playbooks.js';
import {createTeachingReviewClient} from '../../packages/client/src/teaching-review.js';
import {createTeachingGrillingClient} from '../../packages/client/src/teaching-grilling.js';
const {candidateDirectory,origin,input}=JSON.parse(await readFile(process.argv[2],'utf8'));
const browser=createPlaybookClient(async(name,args)=>({content:[{type:'text',text:JSON.stringify({success:true,data:args.params.operation==='validate_teaching_candidate'?{valid:true,guideId:args.params.guide.guideId,taskId:args.taskId,executionAllowed:false,operatorReviewQualified:false}:JSON.parse(await readFile(join(candidateDirectory,args.params.guideId+'.json'),'utf8'))})}]}));
const owner=createTeachingReviewClient({candidateDirectory,browser});
const review={...owner,save:async value=>{
 const result=await owner.save(value);
 if(result.status!=='saved')throw Error('fixture_publication_failed');
 process.stdout.write('published-without-reply\n');
 await new Promise(()=>{setInterval(()=>{},1000);});
 return result;
}};
const readOperatorAnswer=async request=>{
 const response=await fetch(origin+'/answers/'+request.answerRef,{headers:{authorization:'Bearer synthetic-receiver'},redirect:'error'});
 if(!response.ok)throw Error('answer_unavailable');return response.json();
};
await createTeachingGrillingClient({review,readOperatorAnswer}).answer(input);
