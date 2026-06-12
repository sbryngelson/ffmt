subroutine test_trailing_comment()

    ! Already-broken dangling continuation + comment (illegal output from old formatter)
    select case (dir)
    case (101)
        ! this was an illegal dangling continuation comment
        x = 2
    end select

    ! Long trailing comment on a call - should move above and wrap
    ! this is a very long trailing comment that exceeds the line length limit and should be moved above the code line rather than
    ! dangling
    call some_long_sub(arg1, arg2)

    ! Short trailing comment that fits - should stay inline
    call some_sub(a, b, c)  ! quick note

    ! Mid-continuation comment: hoisted above, code stays intact comment about args
    call some_sub(arg1, arg2, arg3, arg4)

    ! Mid-continuation comment with multiple following continuations: hoisted explain the split
    x = 1 + 2 + 3 + y + z

end subroutine test_trailing_comment
