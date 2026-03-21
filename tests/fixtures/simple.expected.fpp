module m_test

    use m_types
    implicit none
    private; public :: s_foo
    integer :: x
    real    :: y

contains
    subroutine s_foo(a, b)

        integer, intent(in) :: a, b

        if (a == b)then
            x = a + b
        else
            x = a - b
        end if
        do i = 1, n
            y = y + x
        end do

    end subroutine s_foo

end module m_test
