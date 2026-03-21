#:include 'macros.fpp'
MODULE m_fypp_test
IMPLICIT NONE
CONTAINS
SUBROUTINE s_test()
#:if defined('MFC_OpenACC')
$:GPU_PARALLEL_LOOP(collapse=3)
DO k=0,p
DO j=0,n
DO i=0,m
x(i,j,k)=0.0_wp
END DO
END DO
END DO
$:END_GPU_PARALLEL_LOOP()
#:else
DO k=0,p
DO j=0,n
DO i=0,m
x(i,j,k)=0.0_wp
END DO
END DO
END DO
#:endif
END SUBROUTINE s_test
END MODULE m_fypp_test
