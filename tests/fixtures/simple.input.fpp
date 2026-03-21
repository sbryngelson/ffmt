MODULE m_test
USE m_types
IMPLICIT NONE
private;public :: s_foo
INTEGER::x
REAL   ::   y
CONTAINS
SUBROUTINE s_foo(a,b)
INTEGER,INTENT(IN)::a,b
IF(a==b)THEN
x=a+b
ELSE
x=a-b
END IF
DO i=1,n
y=y+x
END DO
END SUBROUTINE s_foo
END MODULE m_test
