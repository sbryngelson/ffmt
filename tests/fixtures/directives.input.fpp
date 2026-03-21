SUBROUTINE s_test()
INTEGER :: i,j,k
#if defined(MFC_OpenACC)
!$acc parallel loop collapse(3)
DO k=0,p
DO j=0,n
DO i=0,m
x(i,j,k)=y(i,j,k)*z(i,j,k)
END DO
END DO
END DO
!$acc end parallel loop
#endif
END SUBROUTINE s_test
