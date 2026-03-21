SUBROUTINE s_test()
INTEGER :: i



INTEGER :: j
!$acc parallel loop

DO i=1,n
x(i)=0
END DO
END SUBROUTINE s_test
