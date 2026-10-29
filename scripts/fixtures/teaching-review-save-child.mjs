// Public caller interruption fixture. No private storage methods are replaced.
import {readFile} from 'node:fs/promises';
import {createTeachingReviewClient} from '../../packages/client/src/teaching-review.js';
const [directory,mode,inputFile]=process.argv.slice(2);
setInterval(()=>{},1000);
const input=JSON.parse(await readFile(inputFile,'utf8'));
const browser={inspectTeachingDraft:async()=>{throw Error('not used');},validateTeachingCandidate:async({taskId,guide})=>{
 if(mode==='before'){process.stdout.write('validation-wait\n');await new Promise(()=>{});}
 return {valid:true,guideId:guide.guideId,taskId,executionAllowed:false,operatorReviewQualified:false};
}};
const client=createTeachingReviewClient({candidateDirectory:directory,browser});
await client.save(input);
process.stdout.write('committed-no-reply\n');
