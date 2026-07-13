0.1.1 - critical correctness fix: All Lev kernals understimated distances for strings with length > 255. This was due to the missing | mv termn in the Myers D0 recurrence. 
Found by later review of the code by a friend testing "a", "aba" returned 1 instead of 2. 
Fixed all sites this affected (lev and damerau). Added a test for this case going forwards in tests/test_gate.rs