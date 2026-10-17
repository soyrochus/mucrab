# SPDX-License-Identifier: GPL-3.0-only
# Copyright (C) 2026 muCrab contributors
import copy
import json
import unittest
import w05

class BatchProtocolTests(unittest.TestCase):
    def sample(self):
        request=dict(id='batch',names=['a','b'],options=[dict(mask='new-[N]')],apply=True)
        def state(new):
            rows=[dict(name=name,new_name=target) for name,target in zip(request['names'],new)]
            return dict(rows=rows,duplicate_names=False,old_names_conflict=False,rename_enabled=True,changed_count=sum(r['name']!=r['new_name'] for r in rows))
        result=dict(id='batch',status='ok',previews=[state(['a','b']),state(['new-a','new-b'])],applied=True,unchanged_names=[],outcomes=[dict(name='a',new_name='new-a',success=True),dict(name='b',new_name='new-b',success=False)])
        return request,result
    def test_complete_preview_apply_is_accepted_and_zero_missing_reordered_work_is_rejected(self):
        request,result=self.sample();self.assertEqual(w05.decode(json.dumps(result),[request]),[result])
        for requests,text in [([],''),([request],''),([request],json.dumps(result)+'\n'+json.dumps(result))]:
            with self.assertRaises(ValueError):w05.decode(text,requests)
        for key in ['previews','outcomes']:
            bad=copy.deepcopy(result);bad[key].reverse()
            with self.assertRaises(ValueError):w05.decode(json.dumps(bad),[request])
    def test_conflicts_counts_and_unchanged_lists_cannot_be_forged(self):
        request,result=self.sample()
        for field,value in [('changed_count',0),('rename_enabled',False),('duplicate_names',True),('duplicate_names',0)]:
            bad=copy.deepcopy(result);bad['previews'][-1][field]=value
            with self.assertRaises(ValueError):w05.decode(json.dumps(bad),[request])
        bad=copy.deepcopy(result);bad['outcomes'][0]['success']=1
        with self.assertRaises(ValueError):w05.decode(json.dumps(bad),[request])
        bad=copy.deepcopy(result);bad['unchanged_names']=['a']
        with self.assertRaises(ValueError):w05.decode(json.dumps(bad),[request])
    def test_first_conflict_order_matches_original_and_blocked_apply_has_no_outcomes(self):
        self.assertEqual(w05.preview_flags([dict(name='A',new_name='a'),dict(name='a',new_name='a')]),(False,True))
        self.assertEqual(w05.preview_flags([dict(name='a',new_name='x'),dict(name='b',new_name='x')]),(True,False))
        request,result=self.sample();result['previews'][-1].update(rows=[dict(name='a',new_name='x'),dict(name='b',new_name='x')],duplicate_names=True,rename_enabled=False)
        result.update(applied=False,outcomes=[]);w05.decode(json.dumps(result),[request])

if __name__=='__main__':unittest.main()
