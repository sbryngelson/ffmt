subroutine s_test()
    integer :: i


    integer :: j
    !$acc parallel loop
    do i = 1, n
        x(i) = 0
    end do
end subroutine s_test
