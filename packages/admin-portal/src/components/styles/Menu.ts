import { colors } from "@/constants/colors";
import { css } from "@emotion/react";
import styled from "@emotion/styled";
import AddIcon from "@mui/icons-material/Add"
import { NavLink } from "react-router-dom";
import HowToVoteIcon from "@mui/icons-material/HowToVote"
import AddCircleIcon from "@mui/icons-material/AddCircle"

export const  divContainer = css`
flex: 0 0 auto;
width: 1.5rem;
height: 1.5rem;
`;

export const MenuStyles = {

SideMenuContainer: styled.ul`
  display: flex;
  padding-left: 1rem;
  padding-right: 1rem;
  background-color: white;
  text-transform: uppercase;
  font-size: 0.75rem;
  line-height: 1.5rem;
  & > *:not(:last-child) {
    margin-right: 1rem;
  }
`,

SideMenuActiveItem: styled.li<{isArchivedElectionEvents: boolean}>`
padding-left: 1rem;
padding-right: 1rem;
padding-top: 0.5rem;
padding-bottom: 0.5rem;
cursor: pointer;
color: ${({isArchivedElectionEvents}) => (!isArchivedElectionEvents ? colors.brandColor : colors.secondary)};
border-bottom: ${({ isArchivedElectionEvents }) => (!isArchivedElectionEvents ? `2px solid ${colors.brandSuccess}` : 'none')};
`,

SideMenuArchiveItem: styled.li<{isArchivedElectionEvents: boolean}>`
padding-left: 1rem;
padding-right: 1rem;
padding-top: 0.5rem;
padding-bottom: 0.5rem;
cursor: pointer;
color: ${({isArchivedElectionEvents}) => (isArchivedElectionEvents ? colors.brandColor : colors.secondary)};
border-bottom: ${({ isArchivedElectionEvents }) => (isArchivedElectionEvents ? `2px solid ${colors.brandSuccess}` : 'none')};
`,

EmptyStateContainer: styled.div`
  padding: 1rem;
  background-color: white;
`,

TreeLeavesContainer: styled.div`
    display: flex;
    flex-direction: column;
    margin-left: 0.75rem;
`,

CreateElectionContainer: styled.div`
display: flex;
align-items: center;
color: ${colors.secondary};
& > *:not(:last-child) {
  margin-right: 0.5rem;
}
`,

StyledAddIcon: styled(AddIcon)`
flex: 0 0 auto;
`,

StyledNavLink: styled(NavLink)`
flex-grow: 1;
padding-top: 0.375rem;
padding-bottom: 0.375rem;
border-bottom-width: 2px;
border-bottom-color: white;
cursor: pointer;
white-space: nowrap;
overflow: hidden;
text-overflow: ellipsis;

&:hover {
  border-bottom-color: ${colors.secondary};
}
`,

StyledHiddenDiv: styled.div`
${divContainer}
visibility: hidden;
`,

TreeMenuItemContainer: styled.div`
  display: flex;
  text-align: left;
  align-items: center;
  & > *:not(:last-child) {
    margin-right: 0.5rem;
  }
`,

ItemContainer: styled.p`
display: flex;
align-items: center;
& > *:not(:last-child) {
  margin-right: 0.5rem;
}
`,
HowToVoteStyledIcon: styled(HowToVoteIcon)`
color: ${colors.brandColor}
`,
TreeMenuIconContaier: styled.div`
${divContainer}
cursor: pointer;
color: black;
`,
StyledSideBarNavLink: styled(NavLink)`
flex-grow: 1;
padding-top: 0.375rem;
padding-bottom: 0.375rem;
color: black;
border-bottom-width: 2px;
border-bottom-color: white;
cursor: pointer;
white-space: nowrap;
overflow: hidden;
text-overflow: ellipsis;
&:hover {
border-bottom-color: ${colors.brandColor};
}
&.active {
border-bottom-color: ${colors.brandColor};
}
`,

MenuActionContainer: styled.div`
visibility: hidden;
&.group-hover-visible:hover {
visibility: visible;
}
`,
StyledIconContainer: styled.p`
${divContainer}
cursor: pointer
`,

StyledAddCircleIcon: styled(AddCircleIcon)`
color: ${colors.brandColor}
`
}

